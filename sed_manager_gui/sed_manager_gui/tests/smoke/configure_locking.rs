//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::io::Write as _;
use std::sync::Arc;
use std::time::Duration;

use sed_async::{PolyRuntime, SlintRuntime};
use sed_manager::Host;
use sed_manager_gui::{app::App, toast::ToastQueue};
use sed_manager_gui_slint as ui;
use slint::ComponentHandle as _;
use slint::platform::PointerEventButton::Left;

use crate::driver::Driver;
use crate::test_utils::run_test;

const TEST_PASSWORD: &str = "test-password-1234";
const USER1_PASSWORD: &str = "user1-password-1234";

/// This test takes ownership, activates locking, logs into the locking SP as "Admin1" to reach
/// the "Configure locking" page, changes, saves, and erases a locking range, then logs back out.
///
/// This test exercises the following functionality:
/// - Take ownership
/// - Activate locking
/// - Login (to the locking SP)
/// - Listing and writing locking ranges
/// - Erasing locking ranges
/// - Logout
#[test]
fn configure_locking_ranges() {
    run_test(|| async {
        let runtime = Arc::new(PolyRuntime::Slint(SlintRuntime));
        let ui = ui::MainWindow::new().unwrap();
        let notification_queue = ToastQueue::new(ui.clone_strong());
        let host = Arc::new(Host::new(runtime.clone()));
        let _app = App::new(ui.clone_strong(), notification_queue, host, runtime.clone());

        let ui = Driver::new(ui, Duration::from_secs(5));

        scan_devices(&ui).await;
        select_virtual_device(&ui).await;
        take_ownership(&ui).await;
        activate_locking(&ui).await;
        login_admin1(&ui).await;

        // Toggle "Read locking enabled" of the first non-global range (index 0
        // is the Global Range) and save the ranges.
        let checkbox_id = "locking-range-read-lock-enabled-1";
        ui.check_appearance(checkbox_id).await.unwrap();
        assert_eq!(ui.accessible_checked(checkbox_id).unwrap(), Some(false));
        ui.invoke_default_action(checkbox_id).unwrap();

        // Also move the range to an unaligned span. The UI should round the
        // span down to the alignment granularity (8 on the virtual device),
        // otherwise the device rejects the update.
        // The sectors are 512 bytes, so 1000 and 2000 LBAs are displayed as
        // 500 and 1000 KiB, whereas the unaligned values would be displayed as
        // 500.5 and 1001.5 KiB.
        ui.enter_text("locking-range-start-1", "1001").await.unwrap();
        ui.enter_text("locking-range-end-1", "2003").await.unwrap();
        assert_eq!(ui.accessible_value("locking-range-start-1").unwrap().as_deref(), Some("500 KiB"));
        assert_eq!(ui.accessible_value("locking-range-end-1").unwrap().as_deref(), Some("1000 KiB"));

        // Background commands started by the login may briefly make the session
        // busy, which disables the button. The click is therefore invoked
        // synchronously right after it's found enabled, as opposed to an
        // asynchronous mouse click, during which it may get disabled again.
        ui.check_enabled("save-ranges-submit").await.unwrap();
        ui.invoke_default_action("save-ranges-submit").unwrap();
        ui.check_appearance_toast("Locking ranges written to device").await.unwrap();

        // The ranges are reloaded from the device after saving, so once there
        // are no unsaved changes, the UI reflects the state of the device.
        ui.check_condition("save-ranges-submit", |element| element.accessible_enabled() == Some(false))
            .await
            .unwrap();

        // Reloading replaces the model, and the rows are recreated lazily.
        ui.check_appearance(checkbox_id).await.unwrap();
        assert_eq!(ui.accessible_checked(checkbox_id).unwrap(), Some(true));
        assert_eq!(ui.accessible_value("locking-range-start-1").unwrap().as_deref(), Some("500 KiB"));
        assert_eq!(ui.accessible_value("locking-range-end-1").unwrap().as_deref(), Some("1000 KiB"));

        // Erase the same range, and confirm it in the popup. Erasing is only
        // possible when the range has no unsaved changes, which is the case
        // after the reload.
        ui.check_enabled("locking-range-erase-1").await.unwrap();
        ui.invoke_default_action("locking-range-erase-1").unwrap();
        ui.check_enabled("confirm-popup-submit").await.unwrap();
        ui.invoke_default_action("confirm-popup-submit").unwrap();
        ui.check_appearance_toast("Locking range erased").await.unwrap();
        ui.check_disappearance("confirm-popup-submit").await.unwrap();

        // Erasing only replaces the range's media encryption key, the range's
        // settings must stay the same.
        assert_eq!(ui.accessible_checked(checkbox_id).unwrap(), Some(true));
        assert_eq!(ui.accessible_value("locking-range-start-1").unwrap().as_deref(), Some("500 KiB"));
        assert_eq!(ui.accessible_value("locking-range-end-1").unwrap().as_deref(), Some("1000 KiB"));

        logout(&ui).await;
    });
}

/// This test takes ownership, activates locking, logs into the locking SP as "Admin1" to reach
/// the "Configure locking" page, enables an authority and changes its password, then logs back
/// out and logs in as the newly enabled authority with its new password.
///
/// This test exercises the following functionality:
/// - Take ownership
/// - Activate locking
/// - Login (to the locking SP)
/// - Listing and writing authorities
/// - Changing the password of authorities
/// - Logout
#[test]
fn configure_authorities() {
    run_test(|| async {
        let runtime = Arc::new(PolyRuntime::Slint(SlintRuntime));
        let ui = ui::MainWindow::new().unwrap();
        let notification_queue = ToastQueue::new(ui.clone_strong());
        let host = Arc::new(Host::new(runtime.clone()));
        let _app = App::new(ui.clone_strong(), notification_queue, host, runtime.clone());

        let ui = Driver::new(ui, Duration::from_secs(5));

        scan_devices(&ui).await;
        select_virtual_device(&ui).await;
        take_ownership(&ui).await;
        activate_locking(&ui).await;
        login_admin1(&ui).await;

        // Switch to the "Authorities" tab and enable "User1", which is
        // disabled by default on a freshly-activated locking SP.
        ui.single_click_by_label("Authorities", Left).await.unwrap();
        let checkbox_id = "authority-enabled-User1";
        ui.check_appearance(checkbox_id).await.unwrap();
        assert_eq!(ui.accessible_checked(checkbox_id).unwrap(), Some(false));
        assert_eq!(ui.accessible_enabled("save-authorities-submit").unwrap(), Some(false));
        assert_eq!(ui.accessible_enabled("discard-authorities-submit").unwrap(), Some(false));

        // Changing an authority makes the changes discardable, and discarding
        // them restores the original state.
        ui.invoke_default_action(checkbox_id).unwrap();
        ui.check_enabled("discard-authorities-submit").await.unwrap();
        ui.invoke_default_action("discard-authorities-submit").unwrap();
        ui.check_appearance(checkbox_id).await.unwrap();
        assert_eq!(ui.accessible_checked(checkbox_id).unwrap(), Some(false));
        assert_eq!(ui.accessible_enabled("save-authorities-submit").unwrap(), Some(false));
        assert_eq!(ui.accessible_enabled("discard-authorities-submit").unwrap(), Some(false));

        // Change it again, this time for real.
        ui.invoke_default_action(checkbox_id).unwrap();

        // Background commands started by the login may briefly make the session
        // busy, which disables the button. The click is therefore invoked
        // synchronously right after it's found enabled, so the button can't get
        // disabled by background commands mid-click.
        ui.check_enabled("save-authorities-submit").await.unwrap();
        ui.invoke_default_action("save-authorities-submit").unwrap();
        ui.check_appearance_toast("Authorities written to device").await.unwrap();

        // The authorities are reloaded from the device after saving, so once
        // there are no unsaved changes, the UI reflects the state of the
        // device. The reload must not end the session, so the user must stay
        // logged in.
        ui.check_condition("save-authorities-submit", |element| element.accessible_enabled() == Some(false))
            .await
            .unwrap();
        ui.check_appearance("logout-submit").await.unwrap();
        ui.check_appearance(checkbox_id).await.unwrap();
        assert_eq!(ui.accessible_checked(checkbox_id).unwrap(), Some(true));

        // Change the password of "User1" via the password popup.
        ui.invoke_default_action("authority-change-password-User1").unwrap();
        ui.check_appearance("password-popup-submit").await.unwrap();
        ui.enter_text("new-password", USER1_PASSWORD).await.unwrap();
        ui.enter_text("repeat-password", USER1_PASSWORD).await.unwrap();
        ui.check_enabled("password-popup-submit").await.unwrap();
        ui.invoke_default_action("password-popup-submit").unwrap();
        ui.check_appearance_toast("Password changed").await.unwrap();
        ui.check_disappearance("password-popup-submit").await.unwrap();

        logout(&ui).await;

        // Only enabled authorities are listed in the login panel, so "User1"
        // is now available to log in with.
        ui.single_click("login-authority", Left).await.unwrap();
        ui.select_combo_box_item("login-authority", "User1").unwrap();
        assert_eq!(ui.accessible_value("login-authority").unwrap().as_deref(), Some("User1"));

        // Logging in as "User1" with the new password confirms that the
        // password change went through.
        ui.enter_text("login-password", USER1_PASSWORD).await.unwrap();
        ui.check_enabled("login-submit").await.unwrap();
        ui.invoke_default_action("login-submit").unwrap();
        ui.check_appearance("logout-submit").await.unwrap();

        logout(&ui).await;
    });
}

/// This test takes ownership, activates locking, logs into the locking SP as "Admin1" to reach
/// the "Configure locking" page, and enables the shadow MBR. It then logs out and back in to
/// check that the change was written to the device.
///
/// This test exercises the following functionality:
/// - Take ownership
/// - Activate locking
/// - Login (to the locking SP)
/// - Reading and writing the MBR control
/// - Logout
#[test]
fn configure_mbr_control() {
    run_test(|| async {
        let runtime = Arc::new(PolyRuntime::Slint(SlintRuntime));
        let ui = ui::MainWindow::new().unwrap();
        let notification_queue = ToastQueue::new(ui.clone_strong());
        let host = Arc::new(Host::new(runtime.clone()));
        let _app = App::new(ui.clone_strong(), notification_queue, host, runtime.clone());

        let ui = Driver::new(ui, Duration::from_secs(5));

        scan_devices(&ui).await;
        select_virtual_device(&ui).await;
        take_ownership(&ui).await;
        activate_locking(&ui).await;
        login_admin1(&ui).await;

        // Switch to the "Shadow MBR" tab. The shadow MBR is disabled by
        // default on a freshly-activated locking SP. The switches are only
        // enabled once the MBR status has been loaded and the session is idle.
        ui.single_click_by_label("Shadow MBR", Left).await.unwrap();
        let switch_id = "mbr-enabled";
        ui.check_enabled(switch_id).await.unwrap();
        assert_eq!(ui.accessible_checked(switch_id).unwrap(), Some(false));

        // Flipping the switch writes the change to the device immediately.
        // Same as for the save buttons, the toggle is invoked synchronously so
        // that the switch can't get disabled by background commands mid-click.
        ui.invoke_default_action(switch_id).unwrap();
        ui.check_appearance_toast("MBR control updated").await.unwrap();

        // The MBR status is reloaded from the device after writing, so once
        // the switch is enabled again, the UI reflects the state of the device.
        ui.check_enabled(switch_id).await.unwrap();
        assert_eq!(ui.accessible_checked(switch_id).unwrap(), Some(true));

        // Log out and back in, so that the MBR status is read from the device
        // in a fresh session.
        logout(&ui).await;
        login_admin1(&ui).await;
        ui.single_click_by_label("Shadow MBR", Left).await.unwrap();
        ui.check_enabled(switch_id).await.unwrap();
        assert_eq!(ui.accessible_checked(switch_id).unwrap(), Some(true));

        logout(&ui).await;
    });
}

/// This test takes ownership, activates locking, logs into the locking SP as "Admin1" to reach
/// the "Configure locking" page, and uploads an image to the shadow MBR by typing in its path.
///
/// This test exercises the following functionality:
/// - Take ownership
/// - Activate locking
/// - Login (to the locking SP)
/// - Writing the shadow MBR
/// - Logout
#[test]
fn upload_mbr() {
    // The image spans multiple of the UI's 1 MiB chunks. Its size is a prime
    // number, so it can't line up with the chunks, or any other power-of-two
    // transfer size, and the last chunk is always a partial one. The contents
    // repeat with a prime period too, so misplaced chunks would corrupt the
    // image.
    let image_len = 2_999_999;
    let image: Vec<u8> = (0..image_len).map(|i: u32| (i % 251) as u8).collect();
    let mut image_file = tempfile::NamedTempFile::new().unwrap();
    image_file.write_all(&image).unwrap();
    image_file.flush().unwrap();
    let image_path = image_file.path().to_str().unwrap().to_owned();

    run_test(move || async move {
        let runtime = Arc::new(PolyRuntime::Slint(SlintRuntime));
        let ui = ui::MainWindow::new().unwrap();
        let notification_queue = ToastQueue::new(ui.clone_strong());
        let host = Arc::new(Host::new(runtime.clone()));
        let _app = App::new(ui.clone_strong(), notification_queue, host, runtime.clone());

        let ui = Driver::new(ui, Duration::from_secs(5));

        scan_devices(&ui).await;
        select_virtual_device(&ui).await;
        take_ownership(&ui).await;
        activate_locking(&ui).await;
        login_admin1(&ui).await;

        // Switch to the "Shadow MBR" tab, and switch from the file picker to
        // the text field. The OS's file picker can't be driven by the test.
        ui.single_click_by_label("Shadow MBR", Left).await.unwrap();
        ui.check_enabled("mbr-toggle-file-picker").await.unwrap();
        ui.invoke_default_action("mbr-toggle-file-picker").unwrap();
        ui.enter_text("mbr-image-path", &image_path).await.unwrap();
        assert_eq!(ui.accessible_value("mbr-image-path").unwrap().as_deref(), Some(image_path.as_str()));

        // Same as for the save buttons, the upload is invoked synchronously so
        // that the button can't get disabled by background commands mid-click.
        ui.check_enabled("mbr-upload-submit").await.unwrap();
        ui.invoke_default_action("mbr-upload-submit").unwrap();
        ui.check_appearance_toast("MBR image written").await.unwrap();

        // Once the upload is finished, the button turns back from "Cancel" to
        // "Upload", and becomes usable again.
        ui.check_enabled("mbr-upload-submit").await.unwrap();
        ui.check_condition("mbr-upload-submit", |element| element.accessible_label().as_deref() == Some("Upload"))
            .await
            .unwrap();

        logout(&ui).await;
    });

    drop(image_file);
}

/// This test takes ownership, activates locking, logs into the locking SP as "Admin1" to reach
/// the "Configure locking" page, and cancels an upload to the shadow MBR. It then cancels an
/// upload too late, after it has finished, and checks that the late cancellation doesn't
/// affect the next upload.
///
/// The uploads are read from a named pipe, which lets the test control when the upload
/// proceeds and when it finishes. This makes the cancellation deterministic, as opposed to
/// racing a large file.
///
/// This test exercises the following functionality:
/// - Take ownership
/// - Activate locking
/// - Login (to the locking SP)
/// - Writing the shadow MBR
/// - Cancelling the writing of the shadow MBR
/// - Logout
#[cfg(target_os = "linux")]
#[test]
fn cancel_mbr_upload() {
    let dir = tempfile::tempdir().unwrap();
    let fifo_path = dir.path().join("image.fifo");
    rustix::fs::mknodat(
        rustix::fs::CWD,
        &fifo_path,
        rustix::fs::FileType::Fifo,
        rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR,
        0,
    )
    .unwrap();
    let fifo_path = fifo_path.to_str().unwrap().to_owned();

    let mut image_file = tempfile::NamedTempFile::new().unwrap();
    image_file.write_all(&[0xA5; 4099]).unwrap();
    image_file.flush().unwrap();
    let image_path = image_file.path().to_str().unwrap().to_owned();

    run_test(move || async move {
        let runtime = Arc::new(PolyRuntime::Slint(SlintRuntime));
        let ui = ui::MainWindow::new().unwrap();
        let notification_queue = ToastQueue::new(ui.clone_strong());
        let host = Arc::new(Host::new(runtime.clone()));
        let _app = App::new(ui.clone_strong(), notification_queue, host, runtime.clone());

        let ui = Driver::new(ui, Duration::from_secs(5));

        scan_devices(&ui).await;
        select_virtual_device(&ui).await;
        take_ownership(&ui).await;
        activate_locking(&ui).await;
        login_admin1(&ui).await;

        ui.single_click_by_label("Shadow MBR", Left).await.unwrap();
        ui.check_enabled("mbr-toggle-file-picker").await.unwrap();
        ui.invoke_default_action("mbr-toggle-file-picker").unwrap();
        ui.enter_text("mbr-image-path", &fifo_path).await.unwrap();

        // Start uploading from the pipe, but don't write anything yet. The
        // button turns into "Cancel" only after the upload has checked for
        // cancel requests and started reading the pipe. The read blocks until
        // the test writes to or closes the pipe, so the upload can't finish
        // while the test cancels it.
        let writer = spawn_fifo_writer(&fifo_path);
        ui.check_enabled("mbr-upload-submit").await.unwrap();
        ui.invoke_default_action("mbr-upload-submit").unwrap();
        check_mbr_upload_button(&ui, "Cancel").await;
        ui.invoke_default_action("mbr-upload-submit").unwrap();

        // Writing to the pipe unblocks the read, and the cancel request is
        // picked up before reading the next chunk. The pipe must not be closed
        // before that, or the upload would finish successfully instead.
        writer.send(vec![0x5A; 4099]).unwrap();
        ui.check_appearance_toast("Failed to write MBR image").await.unwrap();
        check_mbr_upload_button(&ui, "Upload").await;
        drop(writer);

        // Start uploading from the pipe again, but this time, close the pipe
        // after cancelling. The upload finishes successfully, since the cancel
        // request comes too late, when there is nothing more to read.
        let writer = spawn_fifo_writer(&fifo_path);
        ui.check_enabled("mbr-upload-submit").await.unwrap();
        ui.invoke_default_action("mbr-upload-submit").unwrap();
        check_mbr_upload_button(&ui, "Cancel").await;
        ui.invoke_default_action("mbr-upload-submit").unwrap();
        drop(writer);
        ui.check_appearance_toast("MBR image written").await.unwrap();
        check_mbr_upload_button(&ui, "Upload").await;

        // The late cancel request must not cancel the next upload. Wait for the
        // previous toasts to go away, so that they can't be mistaken for the
        // outcome of the next upload.
        ui.check_disappearance_toast("MBR image written").await.unwrap();
        ui.check_disappearance_toast("Failed to write MBR image").await.unwrap();
        ui.enter_text("mbr-image-path", &image_path).await.unwrap();
        ui.check_enabled("mbr-upload-submit").await.unwrap();
        ui.invoke_default_action("mbr-upload-submit").unwrap();
        ui.check_appearance_toast("MBR image written").await.unwrap();
        check_mbr_upload_button(&ui, "Upload").await;

        logout(&ui).await;
    });

    drop(image_file);
}

/// Opens the named pipe for writing on a separate thread, since opening blocks until the
/// other end is opened for reading. The data sent to the returned channel is written to
/// the pipe, and the pipe is closed when the channel is dropped.
#[cfg(target_os = "linux")]
fn spawn_fifo_writer(path: &str) -> std::sync::mpsc::Sender<Vec<u8>> {
    let (tx, rx) = std::sync::mpsc::channel::<Vec<u8>>();
    let path = path.to_owned();
    std::thread::spawn(move || {
        let mut fifo = std::fs::OpenOptions::new().write(true).open(path).unwrap();
        for data in rx {
            // The reader may have closed the pipe already, which is not an
            // error for the test.
            let _ = fifo.write_all(&data);
        }
    });
    tx
}

#[cfg(target_os = "linux")]
async fn check_mbr_upload_button(ui: &Driver<sed_manager_gui_slint::MainWindow>, label: &str) {
    ui.check_condition("mbr-upload-submit", |element| {
        element.accessible_enabled() == Some(true) && element.accessible_label().as_deref() == Some(label)
    })
    .await
    .unwrap();
}

async fn scan_devices(ui: &Driver<sed_manager_gui_slint::MainWindow>) {
    // Scan for devices; the virtual device is always included in debug builds.
    ui.single_click("scan-button", Left).await.unwrap();
    ui.check_appearance("device-side-bar-item-0").await.unwrap();
}

async fn select_virtual_device(ui: &Driver<sed_manager_gui_slint::MainWindow>) {
    // Select the virtual device.
    ui.single_click("device-side-bar-item-0", Left).await.unwrap();
    ui.check_appearance("activity-take-ownership").await.unwrap();
}

async fn take_ownership(ui: &Driver<sed_manager_gui_slint::MainWindow>) {
    // Switch to the "Take ownership" activity and fill in the password form.
    ui.single_click("activity-take-ownership", Left).await.unwrap();
    ui.enter_text("new-password", TEST_PASSWORD).await.unwrap();
    ui.enter_text("repeat-password", TEST_PASSWORD).await.unwrap();

    // Submit and wait for the success toast. The submit buttons are disabled while the TPer
    // or the session is busy, so they are invoked synchronously right after they're found
    // enabled, as opposed to an asynchronous mouse click, during which they may get disabled.
    ui.check_enabled("take-ownership-submit").await.unwrap();
    ui.invoke_default_action("take-ownership-submit").unwrap();
    ui.check_appearance_toast("Taken ownership").await.unwrap();

    // A successful action triggers a background re-discovery, which navigates the sidebar
    // back to the "Discovery" activity once it completes. Wait for that before switching
    // activities again, or the switch may be undone by the pending re-discovery.
    ui.check_disappearance("new-password").await.unwrap();
}

async fn activate_locking(ui: &Driver<sed_manager_gui_slint::MainWindow>) {
    // Switch to the "Activate locking" activity and submit the owner password.
    ui.single_click("activity-activate-locking", Left).await.unwrap();
    ui.enter_text("activate-locking-password", TEST_PASSWORD).await.unwrap();
    ui.check_enabled("activate-locking-submit").await.unwrap();
    ui.invoke_default_action("activate-locking-submit").unwrap();
    ui.check_appearance_toast("Locking activated").await.unwrap();
    ui.check_disappearance("activate-locking-password").await.unwrap();
}

async fn login_admin1(ui: &Driver<sed_manager_gui_slint::MainWindow>) {
    // Switch to the "Configure locking" activity. Selecting it triggers a silent, background
    // listing of the locking SP's authorities; wait for that to populate the login panel's
    // authority combo box before interacting with it (there's no toast to wait on here, since
    // the listing is silent).
    ui.check_appearance("activity-configure-locking").await.unwrap();
    ui.single_click("activity-configure-locking", Left).await.unwrap();
    ui.check_condition("login-authority", |element| element.accessible_value().is_some_and(|value| !value.is_empty()))
        .await
        .unwrap();

    // Log in as "Admin1". The authority combo box already defaults to it, since it's the only
    // individual authority enabled by default on a freshly-activated locking SP. Activating
    // locking copies the current SID password into Admin1's credential, so it's the same
    // password used to take ownership, not the virtual device's raw factory default.
    ui.enter_text("login-password", TEST_PASSWORD).await.unwrap();
    ui.check_enabled("login-submit").await.unwrap();
    ui.invoke_default_action("login-submit").unwrap();

    // A successful login switches the "Configure locking" page from the login panel to the
    // locking SP's tabs, landing on "Locking ranges" first, which has its own "Logout" button.
    // Wait for that before proceeding, so the login is confirmed to actually have gone through.
    ui.check_appearance("logout-submit").await.unwrap();
}

async fn logout(ui: &Driver<sed_manager_gui_slint::MainWindow>) {
    // Log back out and wait for the login panel to reappear.
    ui.invoke_default_action("logout-submit").unwrap();
    ui.check_appearance("login-submit").await.unwrap();
}
