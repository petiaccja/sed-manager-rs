use std::rc::Rc;

use crate::{backend::Backend, frontend::Frontend, utility::PeekCell};

//mod access_control;
//mod mbr_editor;
mod range_editor;
mod single_step;
mod user_editor;

pub fn init(frontend: &Frontend, num_devices: usize) {
    single_step::init(frontend, num_devices);
    user_editor::init(frontend, num_devices);
    range_editor::init(frontend, num_devices);
    //access_control::init(frontend, num_devices);
    //mbr_editor::init(frontend, num_devices);
}

pub fn clear(frontend: &Frontend) {
    single_step::clear(frontend);
    user_editor::clear(frontend);
    range_editor::clear(frontend);
    //access_control::clear(frontend);
    //mbr_editor::clear(frontend);
}

pub fn set_callbacks(backend: Rc<PeekCell<Backend>>, frontend: Frontend) {
    single_step::set_callbacks(backend.clone(), frontend.clone());
    user_editor::set_callbacks(backend.clone(), frontend.clone());
    range_editor::set_callbacks(backend.clone(), frontend.clone());
    //access_control::set_callbacks(backend.clone(), frontend.clone());
    //mbr_editor::set_callbacks(backend.clone(), frontend.clone());
}
