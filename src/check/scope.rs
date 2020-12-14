// Copyright Ion Fusion contributors. All Rights Reserved.
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

pub trait Env: ::std::fmt::Debug {
    fn contains(&self, symbol: &str) -> bool;
    fn bind_top_level(&self, symbol: String);
}

pub trait NewScope {
    fn new_scope(self) -> Self;
}

pub type EnvRc = Rc<RefCell<dyn Env>>;

#[derive(new, Debug)]
pub struct Scope {
    parent: EnvRc,
    bindings: RefCell<HashSet<String>>,
}
impl Scope {
    pub fn bind(&self, symbol: String) {
        self.bindings.borrow_mut().insert(symbol);
    }
}
impl NewScope for Rc<RefCell<Scope>> {
    fn new_scope(self) -> Rc<RefCell<Scope>> {
        Rc::new(RefCell::new(Scope::new(
            self.clone(),
            RefCell::new(HashSet::new()),
        )))
    }
}
impl Env for Scope {
    fn contains(&self, symbol: &str) -> bool {
        if self.parent.borrow().contains(symbol) {
            return true;
        }
        self.bindings
            .borrow()
            .iter()
            .any(|binding| binding == symbol)
    }

    fn bind_top_level(&self, symbol: String) {
        self.parent.borrow().bind_top_level(symbol);
    }
}

pub type ScopeRc = Rc<RefCell<Scope>>;

#[derive(new, Debug)]
pub struct BindingEnv {
    pub top_level: RefCell<HashSet<String>>,
}
impl BindingEnv {
    pub fn scope(self) -> Rc<RefCell<Scope>> {
        Rc::new(RefCell::new(Scope::new(
            Rc::new(RefCell::new(self)),
            RefCell::new(HashSet::new()),
        )))
    }
}
impl Env for BindingEnv {
    fn contains(&self, symbol: &str) -> bool {
        if self.top_level.borrow().contains(symbol) {
            return true;
        }
        false
    }

    fn bind_top_level(&self, symbol: String) {
        self.top_level.borrow_mut().insert(symbol);
    }
}
