extern crate rand;
extern crate typemap;

use rand::Rand;
use rand::Rng;
use rand::ThreadRng;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::Weak;
use typemap::ShareMap;

pub trait Entity {
    fn property_map(&mut self) -> &mut ShareMap;
    fn run_self(&mut self) -> bool;
}

pub trait Controller {
    fn run_on(&mut self, entity: &mut ShareMap, global_state: &mut ShareMap);
}

pub struct XGEngine<I = u64, R = ThreadRng> {
    pub entities: BTreeMap<I, Arc<Mutex<Entity>>>,
    pub controllers: BTreeMap<I, (Arc<Mutex<Controller>>, BTreeMap<I, Weak<Mutex<Entity>>>)>,
    pub rng_source: R,
    pub global_state: ShareMap
}

impl XGEngine {
    pub fn new() -> XGEngine {
        XGEngine::from_rng(rand::thread_rng())
    }
}

impl<I, R> XGEngine<I, R>
    where I: Rand + Ord + Clone,
          R: Rng
{
    pub fn from_rng(r: R) -> XGEngine<I, R> {
        XGEngine {
            entities: BTreeMap::new(),
            controllers: BTreeMap::new(),
            rng_source: r,
            global_state: ShareMap::custom(),
        }
    }

    pub fn entity(&mut self, entity: Arc<Mutex<Entity>>) -> I {
        let mut key = self.rng_source.gen::<I>();
        loop {
            if self.entities.contains_key(&key) {
                key = self.rng_source.gen::<I>();
            } else {
                self.entities.insert(key.clone(), entity);
                return key;
            }
        }
    }

    pub fn controller(&mut self, controller: Arc<Mutex<Controller>>) -> I {
        let mut key = self.rng_source.gen::<I>();
        loop {
            if self.controllers.contains_key(&key) {
                key = self.rng_source.gen::<I>();
            } else {
                self.controllers.insert(key.clone(), (controller, BTreeMap::new()));
                return key;
            }
        }
    }

    pub fn register(&mut self, controller_id: I, entity_id: I) -> Result<(), RegisterError> {
        match (self.controllers.get_mut(&controller_id), self.entities.get(&entity_id)) {
            (Some(&mut (_, ref mut registrant)), Some(ref entity)) => {
                registrant.insert(entity_id.clone(), Arc::downgrade(entity));
                Ok(())  
            },
            (Some(_), None) => Err(RegisterError::NoEntityID),
            (None, Some(_)) => Err(RegisterError::NoControllerID),
            (None, None) => Err(RegisterError::NoControllerOrEntityID),
        }
    }

    pub fn run_all(&mut self) {
        for (_, &mut (ref mut controller, ref mut registrant)) in self.controllers.iter_mut() {
            for (_, entity) in registrant.iter() {
                if let Some(entity) = entity.upgrade() {
                    match (entity.lock(), controller.lock()) {
                        (Ok(mut entity), Ok(mut controller)) => {
                            controller.run_on(entity.property_map(), &mut self.global_state);
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

pub enum RegisterError {
    NoControllerID,
    NoEntityID,
    NoControllerOrEntityID,
}
