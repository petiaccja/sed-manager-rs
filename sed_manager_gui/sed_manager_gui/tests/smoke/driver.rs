use std::time::Duration;

use i_slint_backend_testing::{ElementHandle, ElementRoot};
use slint::{
    ComponentHandle, SharedString, Window,
    platform::{Key, PointerEventButton, WindowEvent},
};

use crate::{element_handle_ext::ElementHandleEx as _, test_utils::sleep_until_condition};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("The element id=\"{id}\" not found")]
    ElementNotFound { id: String },
    #[error("The element id=\"{id}\" is ambiguous")]
    AmbiguousElement { id: String },
    #[error("The element title=\"{title}\" not found")]
    ToastNotFound { title: String },
    #[error("The element id=\"{id}\" is not enabled (although present)")]
    ElementNotEnabled { id: String },
}

pub struct Driver<Root: ElementRoot> {
    root: Root,
    timeout: Duration,
}

impl<Root: ElementRoot + ComponentHandle> Driver<Root> {
    pub fn new(root: Root, timeout: Duration) -> Self {
        prevent_clipping(&root);
        Self { root, timeout }
    }

    /// Find a unique element given by its `accessible-id`.
    pub fn find_unique(&self, element_id: &str) -> Result<ElementHandle, Error> {
        let mut it = ElementHandle::find_by_accessible_id(&self.root, element_id).fuse();
        let element = it.next().ok_or_else(|| Error::ElementNotFound { id: element_id.into() })?;
        if it.next().is_some() {
            return Err(Error::AmbiguousElement { id: element_id.into() });
        }
        Ok(element)
    }

    /// Send a single click to the element given by its `accessible-id`.
    pub async fn single_click(&self, element_id: &str, button: PointerEventButton) -> Result<(), Error> {
        self.find_unique(element_id)?.single_click(button).await;
        Ok(())
    }

    /// Type text letter by letter and press enter.
    pub async fn enter_text(&self, element_id: &str, text: &str) -> Result<(), Error> {
        self.single_click(element_id, PointerEventButton::Left).await?;

        let window = self.root.window();

        // Select all, so that the typed text replaces the current one.
        press(window, Key::Control.into());
        tap(window, "a".into());
        release(window, Key::Control.into());

        for ch in text.chars() {
            tap(window, ch.to_string().into());
        }
        tap(window, Key::Return.into());

        Ok(())
    }

    /// Selects an item from a combo box's popup, which must already be open.
    ///
    /// `i-slint-backend-testing` reports the position of elements inside a popup relative to the
    /// popup's own origin rather than to the window, so a plain `ElementHandle::single_click()` on
    /// the popup item misses it. Composing the combo box's own (correctly reported) absolute position
    /// with the item's (incorrectly relative) position happens to cancel out the error and land back
    /// on the right spot.
    pub fn select_combo_box_item(&self, combo_box_id: &str, item_label: &str) -> Result<(), Error> {
        let combo_box_pos = self.find_unique(combo_box_id)?.absolute_position();
        let item = ElementHandle::find_by_accessible_label(&self.root, item_label).next().unwrap();
        let item_pos = item.absolute_position();
        let item_size = item.size();
        let click_pos = slint::LogicalPosition::new(
            combo_box_pos.x + item_pos.x + item_size.width / 2.0,
            combo_box_pos.y + item_pos.y + item_size.height / 2.0,
        );

        let window = self.root.window();
        window.dispatch_event(slint::platform::WindowEvent::PointerMoved { position: click_pos });
        window.dispatch_event(slint::platform::WindowEvent::PointerPressed {
            position: click_pos,
            button: PointerEventButton::Left,
        });
        window.dispatch_event(slint::platform::WindowEvent::PointerReleased {
            position: click_pos,
            button: PointerEventButton::Left,
        });

        Ok(())
    }

    /// Check if the element has appeared within the timeout.
    pub async fn check_appearance(&self, element_id: &str) -> Result<(), Error> {
        let result = sleep_until_condition(|| self.find_unique(element_id).is_ok(), self.timeout).await;
        match result {
            Ok(_) => Ok(()),
            Err(_) => Err(Error::ElementNotFound { id: element_id.into() }),
        }
    }

    /// Check if the element has appeared within the timeout.
    pub async fn check_disappearance(&self, element_id: &str) -> Result<(), Error> {
        let result = sleep_until_condition(|| !self.find_unique(element_id).is_ok(), self.timeout).await;
        match result {
            Ok(_) => Ok(()),
            Err(_) => Err(Error::ElementNotFound { id: element_id.into() }),
        }
    }

    /// Check if the element has appeared within the timeout.
    pub async fn check_appearance_toast(&self, title: &str) -> Result<(), Error> {
        let result = sleep_until_condition(
            || ElementHandle::find_by_accessible_label(&self.root, title).next().is_some(),
            self.timeout,
        )
        .await;
        match result {
            Ok(_) => Ok(()),
            Err(_) => Err(Error::ToastNotFound { title: title.into() }),
        }
    }

    /// Check if the element has appeared within the timeout.
    #[expect(unused)]
    pub async fn check_appearance_with(&self, is_present: impl Fn(&dyn ElementRoot) -> bool) -> Result<(), Error> {
        let result = sleep_until_condition(|| is_present(&self.root), self.timeout).await;
        match result {
            Ok(_) => Ok(()),
            Err(_) => Err(Error::ElementNotFound { id: "<PREDICATE>".into() }),
        }
    }

    /// Check if the element is present and gets enabled within the timeout.
    pub async fn check_enabled(&self, element_id: &str) -> Result<(), Error> {
        let result = sleep_until_condition(
            || self.find_unique(element_id).is_ok_and(|element| element.accessible_enabled() == Some(true)),
            self.timeout,
        )
        .await;
        match result {
            Ok(_) => Ok(()),
            Err(_) => match self.find_unique(element_id) {
                Err(err) => Err(err),
                Ok(_) => Err(Error::ElementNotEnabled { id: element_id.into() }),
            },
        }
    }
}

/// Clipped UI elements cannot be found by the testing backend, which can lead to
/// unexpected test failures. To prevent this the window is resized to large
/// enough so that there is no scrolling or clipping.
fn prevent_clipping(root: &impl ComponentHandle) {
    root.window().set_size(slint::LogicalSize::new(3840.0, 2160.0));
}

fn press(window: &Window, key: SharedString) {
    window.dispatch_event(WindowEvent::KeyPressed { text: key });
}

fn release(window: &Window, key: SharedString) {
    window.dispatch_event(WindowEvent::KeyReleased { text: key });
}

fn tap(window: &Window, key: SharedString) {
    press(window, key.clone());
    release(window, key);
}
