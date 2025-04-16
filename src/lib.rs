use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock, RwLock, RwLockWriteGuard, RwLockReadGuard};
use std::ops::{Deref, DerefMut};

static INSTANCE: OnceLock<Mutex<Di>> = OnceLock::new();

type ThreadSafeAny = Arc<RwLock<dyn Any + Send + Sync + 'static>>;

pub struct Di {
    providers: RwLock<HashMap<TypeId, Arc<dyn Provider>>>,
    single_map: HashMap<TypeId, ThreadSafeAny>,
}

pub struct SingleRef<T> {
    value: Arc<RwLock<T>>,
}

impl<T> SingleRef<T> {
    pub fn get(&self) -> RwLockReadGuard<T> {
        self.value.read().unwrap()
    }

    pub fn get_mut(&mut self) -> RwLockWriteGuard<T> {
        self.value.write().unwrap()
    }
}

impl<T> Clone for SingleRef<T> {
    fn clone(&self) -> Self {
        SingleRef {
            value: self.value.clone(),
        }
    }
}

impl Di {
    fn get_instance() -> &'static Mutex<Di> {
        INSTANCE.get_or_init(|| Mutex::new(Di{
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


pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

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
            let mut config = config.get_mut();
            assert_eq!(config.port, 8080);
            config.port = 8081;
        }
        if let Some(mut config) = Di::get_single::<Configuration>() {
            let mut config = config.get_mut();
            assert_eq!(config.port, 8081);
        }
        
        ()
    }
}
