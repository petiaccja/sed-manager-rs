use std::{
    rc::Rc,
    sync::atomic::{AtomicI32, Ordering},
};

use sed_manager_gui_slint as ui;
use slint::{ComponentHandle, Model, ModelExt, ModelRc, VecModel};

pub struct ToastQueue {
    model: Rc<VecModel<ui::ToastQueueItem>>,
    id: AtomicI32,
}

impl ToastQueue {
    pub fn new(ui: ui::MainWindow) -> Rc<Self> {
        let queue = Rc::from(Self { model: VecModel::from(Vec::new()).into(), id: 0.into() });

        let ui_queue = ui.global::<ui::ToastQueue>();
        ui_queue.set_queue(ModelRc::new(queue.model.clone().reverse()));
        {
            let queue = queue.clone();
            ui_queue.on_push(move |notification| {
                queue.push(notification);
            });
        }
        {
            let queue = queue.clone();
            ui_queue.on_dismiss(move |id| {
                queue.dismiss(id);
            });
        }

        queue
    }

    pub fn push(&self, toast: ui::Toast) {
        let id = self.id.fetch_add(1, Ordering::Relaxed);
        self.model.push(ui::ToastQueueItem { id, toast });
    }

    pub fn dismiss(&self, id: i32) {
        self.model
            .iter()
            .enumerate()
            .find(|(_, item)| item.id == id)
            .map(|(index, _)| self.model.remove(index));
    }
}
