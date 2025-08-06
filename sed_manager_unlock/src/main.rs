//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::fs::File;
use std::io::Write;
use std::sync::Arc;
use std::{io, usize};

use sed_manager::applications::Error as AppError;
use sed_manager::applications::{get_general_lookup, get_locking_sp};
use sed_manager::device::Device;
use sed_manager::messaging::discovery::Discovery;
use sed_manager::rpc::{Error as RPCError, MethodStatus, TokioRuntime};
use sed_manager::spec::column_types::{AuthorityRef, Name};
use sed_manager::spec::core::mbr_control;
use sed_manager::spec::objects::{Authority, LockingRange, MBRControl};
use sed_manager::spec::table_id;
use sed_manager::tper::{Session, TPer};

mod device_list;
mod error;

use device_list::DeviceList;
use error::Error;

fn check_quit(input: &str) -> Result<(), Error> {
    if input == ":q" || input == ":exit" {
        Err(Error::Quit)
    } else {
        Ok(())
    }
}

fn read_line() -> Result<String, Error> {
    let stdin = io::stdin();
    let mut input = String::new();
    let _ = stdin.read_line(&mut input);
    if input.ends_with('\n') {
        input.pop();
    }
    check_quit(&input)?;
    Ok(input)
}

fn wait_for_keypress() {
    println!("Press enter to exit...");
    let _ = read_line();
}

fn select_device(device_list: &DeviceList) -> Result<&(Arc<dyn Device>, Discovery), Error> {
    let list = if !device_list.shadowed.is_empty() {
        device_list.shadowed.as_slice()
    } else {
        device_list.locked.as_slice()
    };

    if list.is_empty() {
        Err(Error::NoDevice)
    } else if list.len() == 1 {
        Ok(&list[0])
    } else {
        for (idx, (device, _)) in list.iter().enumerate() {
            println!("[{}] {} / {}", idx + 1, device.model_number(), device.serial_number());
        }
        loop {
            print!("Select device: ");
            let _ = std::io::stdout().flush();
            let idx: usize = read_line()?.parse().unwrap_or(usize::MAX) - 1;
            if idx < list.len() {
                println!();
                break Ok(&list[idx]);
            }
        }
    }
}

async fn get_user_by_name(name: &str, discovery: &Discovery) -> Result<AuthorityRef, Error> {
    let ssc = discovery.get_primary_ssc().ok_or(AppError::NoAvailableSSC)?;
    let locking_sp = get_locking_sp(ssc.feature_code())?;
    let lookup = get_general_lookup(ssc.feature_code());
    let uid = lookup
        .by_name(name, table_id::AUTHORITY.as_uid(), Some(locking_sp.as_uid()))
        .ok_or(Error::InvalidUser)?;
    AuthorityRef::try_from(uid).map_err(|_| Error::InvalidUser)
}

async fn get_user_by_common_name(name: &str, discovery: &Discovery, tper: &TPer) -> Result<AuthorityRef, Error> {
    if name.is_empty() {
        return Err(Error::InvalidUser);
    }
    let ssc = discovery.get_primary_ssc().ok_or(AppError::NoAvailableSSC)?;
    let locking_sp = get_locking_sp(ssc.feature_code())?;
    let session = tper.start_session(locking_sp, None, None).await?;
    session
        .with(async |session| {
            let authorities = session.next(table_id::AUTHORITY, None, None).await?;
            for authority in authorities {
                let common_name: Name = session.get(authority, Authority::COMMON_NAME).await?;
                if common_name.as_slice() == name.as_bytes() {
                    return Ok(AuthorityRef::try_from(authority).unwrap());
                }
            }
            Err(Error::InvalidUser)
        })
        .await
}

async fn prompt_login(tper: &TPer, discovery: &Discovery) -> Result<Session, Error> {
    let ssc = discovery.get_primary_ssc().ok_or(AppError::NoAvailableSSC)?;
    let locking_sp = get_locking_sp(ssc.feature_code())?;

    print!("  Username: ");
    let _ = std::io::stdout().flush();
    let name = read_line()?;

    let user = get_user_by_name(&name, &discovery)
        .await
        .or(get_user_by_common_name(&name, discovery, tper).await)?;

    let password = rpassword::prompt_password("  Password: ").unwrap();
    check_quit(&password)?;

    tper.start_session(locking_sp, Some(user), Some(password.as_bytes())).await.map_err(|e| e.into())
}

fn print_unlock_result(object: &str, result: Result<(), RPCError>, read: bool, write: bool) {
    let operations = match (read, write) {
        (false, false) => "",
        (true, false) => "R",
        (false, true) => "W",
        (true, true) => "RW",
    };
    match result {
        Ok(_) => println!("- {object}: Unlocked {operations}"),
        Err(RPCError::MethodFailed(MethodStatus::NotAuthorized)) => (),
        Err(error) => println!("{object}: {error}"),
    };
}

async fn unlock_device(session: &Session, discovery: &Discovery) -> Result<(), Error> {
    let ssc = discovery.get_primary_ssc().ok_or(AppError::NoAvailableSSC)?;
    let locking_sp = get_locking_sp(ssc.feature_code())?;
    let lookup = get_general_lookup(ssc.feature_code());

    let mbr_result = session.set(mbr_control::MBR_CONTROL.as_uid(), MBRControl::DONE, true).await;
    print_unlock_result("MBR", mbr_result, false, false);

    let ranges = session.next(table_id::LOCKING, None, None).await?;
    for range in ranges {
        let read_result = session.set(range, LockingRange::READ_LOCKED, false).await;
        let write_result = session.set(range, LockingRange::WRITE_LOCKED, false).await;
        let (read_ok, write_ok) = (read_result.is_ok(), write_result.is_ok());
        let name = lookup.by_uid(range, Some(locking_sp.as_uid())).unwrap_or(range.to_string());
        print_unlock_result(&name, read_result.or(write_result), read_ok, write_ok);
    }

    Ok(())
}

async fn run_pba_sequence(runtime: Arc<TokioRuntime>) -> Result<(), Error> {
    let device_list = DeviceList::query()?;

    #[cfg(debug_assertions)]
    println!("{}\n", &device_list);

    let (device, discovery) = select_device(&device_list)?;
    let tper = TPer::new_on_default_com_id(device.clone(), runtime)?;
    println!("Enter credentials for {} - {}", device.model_number(), device.serial_number());
    let session = loop {
        match prompt_login(&tper, discovery).await {
            Ok(session) => break session,
            Err(Error::Quit) => return Err(Error::Quit),
            Err(error) => println!("{error}"),
        }
    };
    println!();
    println!("Unlocking...");
    let _ = session.with(async |session| unlock_device(&session, discovery).await).await;
    println!();
    Ok(())
}

fn init_logging() -> Option<tracing_appender::non_blocking::WorkerGuard> {
    let temp_dir = std::env::temp_dir();
    let log_file_path = temp_dir.join("sed-manager.log");
    if let Ok(file) = File::create(log_file_path.as_path()) {
        let (non_blocking, guard) = tracing_appender::non_blocking(file);
        let subscriber = tracing_subscriber::fmt()
            .with_ansi(false)
            .with_writer(non_blocking)
            .with_max_level(tracing::Level::DEBUG)
            .with_target(false);
        let _ = tracing::subscriber::set_global_default(subscriber.finish());
        Some(guard)
    } else {
        None
    }
}

#[tokio::main]
async fn main() -> () {
    let runtime = Arc::new(TokioRuntime::new());
    let _guard = init_logging();
    println!("{BANNER}\nWelcome to SEDManager v{VERSION}!\n{USAGE}\n");

    let result = run_pba_sequence(runtime.clone()).await;
    match result {
        Ok(_) => std::thread::sleep(core::time::Duration::from_secs(1)),
        Err(Error::Quit) => (),
        Err(error) => {
            println!("{error}");
            wait_for_keypress();
        }
    }
}

const BANNER: &str = r"
     ____  _____ ____  __  __                                   
    / ___|| ____|  _ \|  \/  | __ _ _ __   __ _  __ _  ___ _ __ 
    \___ \|  _| | | | | |\/| |/ _` | '_ \ / _` |/ _` |/ _ \ '__|
     ___) | |___| |_| | |  | | (_| | | | | (_| | (_| |  __/ |   
    |____/|_____|____/|_|  |_|\__,_|_| |_|\__,_|\__, |\___|_|   
                                                |___/           ";

const VERSION: &str = env!("CARGO_PKG_VERSION");

const USAGE: &str = r"Follow the prompts to unlock your drives.
Enter :q or :exit at any time to quit.";
