extern crate rayon;
extern crate typemap;

use rayon::prelude::*;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::Weak;
use typemap::ShareMap;

pub type PropertyMap = ShareMap;
pub type GlobalState = Arc<Mutex<ShareMap>>;
pub type Map<K, V> = BTreeMap<K, V>;
pub type Set<T> = BTreeSet<T>;

pub trait Entity {
    fn property_map(&mut self) -> &mut PropertyMap;
}
pub type WeakEntity = Weak<Mutex<Entity + Send>>;
pub type StrongEntity = Arc<Mutex<Entity + Send>>;

pub trait Controller {
    fn control(&mut self, entity: StrongEntity, global_state: GlobalState);
}
pub type StrongController = (Arc<Mutex<Box<Controller + Send>>>, Arc<Mutex<Vec<WeakEntity>>>);


pub struct XGEngine {
    pub entities: Map<u64, StrongEntity>,
    pub controllers: Map<u64, StrongController>,
    pub global_state: GlobalState,
    used_entity_keys: Set<u64>,
    next_entity_key: u64,
    used_controller_keys: Set<u64>,
    next_controller_key: u64,
}
impl XGEngine {
    pub fn new() -> XGEngine {
        XGEngine {
            entities: Map::new(),
            controllers: Map::new(),
            global_state: Arc::new(Mutex::new(ShareMap::custom())),
            used_entity_keys: Set::new(),
            next_entity_key: 0,
            used_controller_keys: Set::new(),
            next_controller_key: 0,
        }
    }

    pub fn entity(&mut self, entity: StrongEntity) -> u64 {
        let key = keygen(&self.used_entity_keys, &mut self.next_entity_key);
        self.entities.insert(key, entity);
        key
    }

    pub fn controller(&mut self, controller: StrongController) -> u64 {
        let key = keygen(&self.used_controller_keys, &mut self.next_controller_key);
        self.controllers.insert(key, controller);
        key
    }

    pub fn register(&mut self, controller: StrongController, entity: WeakEntity) {
        controller.1.lock().unwrap().push(entity);
    }

    pub fn register_by_id(&mut self, controller_id: u64, entity_id: u64) -> bool {
        match (self.controllers.get(&controller_id).cloned(), self.entities.get(&entity_id).cloned()) {
            (Some(controller), Some(entity)) => {
                self.register(controller, StrongEntity::downgrade(&entity));
                true
            },
            _ => false,
        }
    }

    pub fn run_all(&mut self) {
        self.controllers.par_iter().for_each(|(_, &(ref controller, ref registrant))|{
            let registrant = registrant.lock().unwrap();
            registrant.par_iter().for_each(|entity|{
                let mut controller = controller.lock().unwrap();
                if let Some(entity) = entity.upgrade() {
                    controller.control(entity, self.global_state.clone());
                }
            });
        });
    }
}

fn keygen(used: &Set<u64>, next: &mut u64) -> u64 {
    let mut key;
    loop {
        key = *next;
        if used.contains(&key) {
            *next += 1;
        } else {
            break;
        }
    }
    key
}
