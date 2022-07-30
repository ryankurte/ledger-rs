use std::error::Error;

use clap::{Parser, Command};
use ledger_transport::{Exchange, APDUErrorCode, APDUCommand, APDUAnswer};
use ledger_transport_tcp::{TransportTcp, TcpOptions};
use ledger_zondax_generic::{DeviceInfo, AppInfo, Version};
use strum::{Display, EnumString, EnumVariantNames};
use log::LevelFilter;

/// Ledger command line utility
#[derive(Clone, PartialEq, Debug, Parser)]
pub struct Options {

    #[clap(subcommand)]
    /// Transport for ledger connection
    transport: Transport,

    /// Enable verbose logging
    #[clap(long, default_value = "debug")]
    level: LevelFilter,
}

#[derive(Clone, PartialEq, Debug, Parser)]
pub enum Commands {
    /// Fetch device info
    DeviceInfo,

    /// Fetch application info
    AppInfo,

    /// Fetch application version
    AppVersion{
        /// Application ADPU class
        cla: u8,
    },
}

#[derive(Clone, PartialEq, Debug, Parser, Display)]
pub enum Transport {
    /// USB HID
    Hid,
    /// Bluetooth Low Energy
    Ble,
    /// TCP (Speculos simulator)
    Tcp{
        #[clap(flatten)]
        opts: TcpOptions,

        #[clap(subcommand)]
        cmd: Commands,
    },
    /// Zemu simulator
    Zemu,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {

    // Parse command line arguments
    let args = Options::parse();

    // Setup logging
    simplelog::SimpleLogger::init(args.level, simplelog::Config::default()).unwrap();

    // Connect to transport and execute commands
    match args.transport {
        Transport::Tcp{ opts, cmd } => {
            let t = TransportTcp::new(opts).await?;

            execute(t, cmd).await?;
        }
        _ => todo!("{} transport not yet implemented", args.transport),
    };

    // Execute command

    println!("Hello, world!");

    Ok(())
}

const INS_GET_VERSION: u8 = 0x00;
const CLA_APP_INFO: u8 = 0xb0;
const INS_APP_INFO: u8 = 0x01;
const CLA_DEVICE_INFO: u8 = 0xe0;
const INS_DEVICE_INFO: u8 = 0x01;

/// Execute a command with the provided transport
async fn execute<T, E>(t: T, cmd: Commands) -> anyhow::Result<()> 
where
    T: Exchange<Error=E>,
    E: Error + Sync + Send + 'static,
{
    // Setup the command ADPU
    let command: APDUCommand<Vec<u8>> = match cmd {
        Commands::DeviceInfo => APDUCommand::new(CLA_DEVICE_INFO, INS_DEVICE_INFO),
        Commands::AppInfo => APDUCommand::new(CLA_APP_INFO, INS_APP_INFO),
        Commands::AppVersion{cla} => APDUCommand::new(cla, INS_GET_VERSION),
        _ => todo!("Command not yet implemented"),
    };

    // Execute request
    let response = t.exchange(&command).await?;
    match response.error_code() {
        Ok(APDUErrorCode::NoError) => {}
        Ok(err) => return Err(anyhow::anyhow!("unhandled APDU response: {:?}", err)),
        Err(err) => return Err(anyhow::anyhow!("unknown APDU response: {:?}", err)),
    }

    // Handle response ADPU
    let response_data = response.data();
    log::debug!("response data: {:?}", response_data);

    match cmd {
        Commands::DeviceInfo => {
            let device_info = DeviceInfo::try_from(response_data)?;
            log::info!("device info: {:#?}", device_info);
        },
        Commands::AppInfo => {
            let app_info = AppInfo::try_from(response_data)?;
            log::info!("app info: {:#?}", app_info);
        },
        Commands::AppVersion{ .. } => {
            let app_version = Version::try_from(response_data)?;
            log::info!("app version: {:#?}", app_version);
        },
        _ => (),
    }

    Ok(())
}