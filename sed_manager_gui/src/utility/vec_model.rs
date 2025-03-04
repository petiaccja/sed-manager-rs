use slint::{Model, ModelRc, VecModel};

pub fn into_vec_model<Item: Clone + 'static>(v: Vec<Item>) -> ModelRc<Item> {
    ModelRc::new(VecModel::from(v))
}

pub fn as_vec_model<Item: Clone + 'static>(model: &ModelRc<Item>) -> &VecModel<Item> {
    model.as_any().downcast_ref::<VecModel<Item>>().expect("expected a VecModel")
}
