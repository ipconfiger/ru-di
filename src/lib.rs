use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock, RwLock, RwLockWriteGuard, RwLockReadGuard};
use std::ops::{Deref, DerefMut};
use tokio::sync::{Mutex as TokioMutex, RwLock as TokioRwLock};

static STD_INSTANCE: OnceLock<Mutex<Di>> = OnceLock::new();
static AYN_INSTANCE: OnceLock<TokioMutex<TkDi>> = OnceLock::new();

type ThreadSafeAny = Arc<RwLock<dyn Any + Send + Sync + 'static>>;

type AsyncSaftAny = Arc<TokioRwLock<dyn Any + Send + Sync + 'static>>;

pub struct Di {
    providers: RwLock<HashMap<TypeId, Arc<dyn Provider>>>,
    single_map: HashMap<TypeId, ThreadSafeAny>,
}

pub struct TkDi {
    providers: TokioRwLock<HashMap<TypeId, Arc<dyn TkProvider>>>,
    async_map: HashMap<TypeId, AsyncSaftAny>,
}


pub struct SingleRef<T> {
    value: Arc<RwLock<T>>,
}

impl<T> SingleRef<T> {
    pub fn get(&self) -> Result<RwLockReadGuard<T>, DiError> {
        self.value.read().map_err(|_| DiError::LockError)
    }

    pub fn get_mut(&mut self) -> Result<RwLockWriteGuard<T>, DiError> {
        self.value.write().map_err(|_| DiError::LockError)
    }
}

impl<T> Clone for SingleRef<T> {
    fn clone(&self) -> Self {
        SingleRef {
            value: self.value.clone(),
        }
    }
}


pub struct SingleAsyncRef<T> {
    value: Arc<TokioRwLock<T>>,
}

impl<T> SingleAsyncRef<T> {
    pub async fn get(&self) -> tokio::sync::RwLockReadGuard<'_, T> {
        self.value.read().await
    }

    pub async fn get_mut(&mut self) -> tokio::sync::RwLockWriteGuard<'_, T> {
        self.value.write().await
    }
}

impl<T> Clone for SingleAsyncRef<T> {
    fn clone(&self) -> Self {
        SingleAsyncRef {
            value: self.value.clone(),
        }
    }
}

impl TkDi {
    fn get_instance() -> &'static TokioMutex<TkDi> {
        AYN_INSTANCE.get_or_init(|| TokioMutex::new(TkDi{
            providers: TokioRwLock::new(HashMap::new()),
            async_map: HashMap::new(),
        }))
    }

    async fn _register<T, F>(&self, factory: F)
    where
        T: 'static + Send + Sync,
        F: Fn(&TkDi) -> T + Send + Sync + 'static,
    {
        let provider = FactoryProvider {
            factory,
            _marker: std::marker::PhantomData,
        };
        let type_id = std::any::TypeId::of::<T>();
        let mut providers = self.providers.write().await;
        providers.insert(type_id, Arc::new(provider));
    }
    
    pub async fn register<T, F>(factory: F)
    where
        T: 'static + Send + Sync,
        F: Fn(&TkDi) -> T + Send + Sync + 'static,
    {
        let di = TkDi::get_instance().lock().await;
        di._register(factory);
    }
    
    pub async fn get_inner<T: 'static>(&self) -> Result<T, Box<dyn std::error::Error>> {
        let type_id = std::any::TypeId::of::<T>();
        let providers = self.providers.read().await;
        let provider = providers.get(&type_id).ok_or("Provider not found")?;
        let any = provider.provide(self);
        // 从 Box<dyn Any> 中提取 Arc<T>
        let t = any.downcast::<T>().map_err(|_| "Downcast failed")?;
        Ok(*t)
    }
    pub async fn get<T: 'static>() -> Result<T, Box<dyn std::error::Error>> {
        let di = TkDi::get_instance().lock().await;
        di.get_inner().await
    }

    fn _register_single<T>(&mut self, instance: T)
    where
        T: 'static + Send + Sync,
    {
        let type_id = std::any::TypeId::of::<T>();
        let any = Arc::new(TokioRwLock::new(instance));
        self.async_map.insert(type_id, any);
    }

    pub async fn register_single<T>(instance: T)
    where
        T: 'static + Send + Sync,
    {
        let mut di = TkDi::get_instance().lock().await;
        di._register_single(instance);
    }

    fn _get_single<T: Any + Send + Sync + 'static>(&self) -> Option<SingleAsyncRef<T>> {
        let type_id = std::any::TypeId::of::<T>();
        let any = self.async_map.get(&type_id)?;
        let value = unsafe {
            let ptr = Arc::into_raw(any.clone());
            Arc::from_raw(ptr as *const TokioRwLock<T>)
        };
        Some(SingleAsyncRef { value })
    }

    pub async fn get_single<T: Any + Send + Sync + 'static>() -> Option<SingleAsyncRef<T>> {
        let di = TkDi::get_instance().lock().await;
        di._get_single::<T>()
    }
    
}


impl Di {
    fn get_instance() -> &'static Mutex<Di> {
        STD_INSTANCE.get_or_init(|| Mutex::new(Di{
            providers: RwLock::new(HashMap::new()),
            single_map: HashMap::new(),
        }))
    }
    
    fn _register_single<T>(&mut self, instance: T)
    where
        T: 'static + Send + Sync,
    {
        let type_id = std::any::TypeId::of::<T>();
        let any = Arc::new(RwLock::new(instance));
        self.single_map.insert(type_id, any);
    }

   
    
    pub fn register_single<T>(instance: T)
    where
        T: 'static + Send + Sync,
    {
        let mut di = Di::get_instance().lock().unwrap();
        di._register_single(instance);
    }
    
    
    fn _register<T, F>(&self, factory: F)
    where
        T: 'static + Send + Sync,
        F: Fn(&Di) -> T + Send + Sync + 'static,
    {
        let provider = FactoryProvider {
            factory,
            _marker: std::marker::PhantomData,
        };
        let type_id = std::any::TypeId::of::<T>();
        let mut providers = self.providers.write().unwrap();
        providers.insert(type_id, Arc::new(provider));
    }
    
    pub fn register<T, F>(factory: F)
    where
        T: 'static + Send + Sync,
        F: Fn(&Di) -> T + Send + Sync + 'static,
    {
        let di = Di::get_instance().lock().unwrap();
        di._register(factory);
    }

     pub fn get_inner<T: 'static>(&self) -> Result<T, Box<dyn std::error::Error>> {
        let type_id = std::any::TypeId::of::<T>();
        let providers = self.providers.read().unwrap();
        let provider = providers.get(&type_id).ok_or("Provider not found")?;

        let any = provider.provide(self);
        // 从 Box<dyn Any> 中提取 Arc<T>
         let t = any.downcast::<T>().map_err(|_| "Downcast failed")?;
         Ok(*t)
    }
    pub fn get<T: 'static>() -> Result<T, Box<dyn std::error::Error>> {
        let di = Di::get_instance().lock().unwrap();
        di.get_inner()
    }
    
    fn _get_single<T: Any + Send + Sync + 'static>(&self) -> Option<SingleRef<T>> {
        let type_id = std::any::TypeId::of::<T>();
        let any = self.single_map.get(&type_id)?;
        if any.type_id() != type_id {
            return None;
        }
        // 安全转换
        let value = unsafe {
            let ptr = Arc::into_raw(any.clone());
            Arc::from_raw(ptr as *const RwLock<T>)
        };
        Some(SingleRef { value })
    }
    pub fn get_single<T: Any + Send + Sync + 'static>() -> Option<SingleRef<T>> {
        let di = Di::get_instance().lock().unwrap();
        di._get_single::<T>()
    }
    
}

trait Provider: Send + Sync {
    fn provide(&self, di: &Di) -> Box<dyn Any>;
}

trait TkProvider: Send + Sync {
    fn provide(&self, di: &TkDi) -> Box<dyn Any>;
}

struct FactoryProvider<F, T> {
    factory: F,
    _marker: std::marker::PhantomData<T>,
}

impl<F, T> Provider for FactoryProvider<F, T>
where
    F: Fn(&Di) -> T + Send + Sync + 'static,
    T: 'static + Send + Sync,
{
    fn provide(&self, di: &Di) -> Box<dyn Any> {
        Box::new((self.factory)(di))
    }
}


impl<F, T> TkProvider for FactoryProvider<F, T>
where
    F: Fn(&TkDi) -> T + Send + Sync + 'static,
    T: 'static + Send + Sync,
{
    fn provide(&self, di: &TkDi) -> Box<dyn Any> {
        Box::new((self.factory)(di))
    }
}

#[derive(Debug)]
pub enum DiError {
    ProviderNotFound,
    TypeMismatch,
    LockError,
}

pub type DiResult<T> = Result<T, DiError>;

#[cfg(test)]
mod tests {
    use super::*;
    
    struct Configuration {
        port: u16,
    }
    
    #[derive(Clone)]
    struct Database {
        port: u16,
    }
    
    #[derive(Clone)]
    struct  AppService {
        db: Database,
    }

    #[tokio::test]
    async fn async_test() {
        TkDi::register::<Database, _>(|_| {
            Database{port: 3306}
        }).await;
        println!("regist database done");
        let db = TkDi::get::<Database>().await.unwrap();
        
        TkDi::register_single(Configuration{port: 8080}).await;
        
        println!("regist app done");
        
        //let result = TkDi::get::<AppService>().await.unwrap();
        
        //assert_eq!(result.db.port, 3306);
        
        if let Some(mut config) = TkDi::get_single::<Configuration>().await {
            let mut config = config.get_mut().await;
            assert_eq!(config.port, 8080);
            config.port = 8081;
        }
        if let Some(mut config) = TkDi::get_single::<Configuration>().await{
            let mut config = config.get_mut().await;
        }
    }

    #[test]
    fn it_works() {
        Di::register::<Database, _>(|_| {
            Database{port: 3306}
        });
        println!("regist database done");
        
        Di::register_single(Configuration{port: 8080});
        
        Di::register::<AppService, _>(|di| {
            let db = di.get_inner::<Database>().unwrap();
            AppService{ db:db.clone()}
        });
        println!("regist app done");
        
        let result = Di::get::<AppService>().unwrap();
        
        assert_eq!(result.db.port, 3306);
        
        if let Some(mut config) = Di::get_single::<Configuration>() {
            let mut config = config.get_mut().unwrap();
            assert_eq!(config.port, 8080);
            config.port = 8081;
        }
        if let Some(mut config) = Di::get_single::<Configuration>() {
            let mut config = config.get_mut().unwrap();
            assert_eq!(config.port, 8081);
        }
        
        ()
    }
}
