use clap::{Parser, Subcommand};
use color_eyre::{Result, eyre::eyre};
use pico_de_gallo_lib::{PicoDeGallo, SpiPhase, SpiPolarity, list_devices};
use std::num::ParseIntError;
use tabled::builder::Builder;
use tabled::settings::object::Rows;
use tabled::settings::{Alignment, Style};

/// Output format for data display.
#[derive(clap::ValueEnum, Clone, Debug, Default)]
pub enum OutputFormat {
    /// Hexadecimal byte dump (default)
    #[default]
    Hex,
    /// Raw binary output (bytes written directly to stdout)
    Binary,
    /// ASCII representation (printable chars shown, others as '.')
    Ascii,
}

#[derive(Parser, Debug)]
#[command(
    name = "Pico De Gallo",
    author = "Felipe Balbi <febalbi@microsoft.com>",
    about = "Access I2C/SPI devices through Pico De Gallo",
    arg_required_else_help = true,
    version
)]
pub struct Cli {
    #[arg(short, long)]
    serial_number: Option<String>,

    /// Output format for read data
    #[arg(short, long, value_enum, default_value_t)]
    format: OutputFormat,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// List all connected Pico de Gallo devices
    List,

    /// Get firmware version
    Version,

    /// I2C access methods
    I2c {
        /// I2C commands
        #[command(subcommand)]
        command: I2cCommands,
    },

    /// SPI access methods
    Spi {
        /// SPI commands
        #[command(subcommand)]
        command: SpiCommands,
    },

    /// Set bus parameters for I2C and SPI
    SetConfig {
        /// I2C frequency
        #[arg(long)]
        i2c_frequency: u32,

        /// SPI frequency
        #[arg(long)]
        spi_frequency: u32,

        /// SPI phase first transition
        #[arg(long, default_value_t)]
        spi_first_transition: bool,

        /// SPI polarity idle low
        #[arg(long, default_value_t)]
        spi_idle_low: bool,
    },
}

#[derive(Subcommand, Debug)]
enum I2cCommands {
    /// Scan I2C bus for existing devices
    Scan {
        /// Attempt reserved addresses
        #[arg(short, long, default_value_t = false)]
        reserved: bool,
    },

    /// Read bytes through the I2C bus from device at given address
    Read {
        /// I2C slave address (7-bit, 0x00–0x7F)
        #[arg(short, long, value_parser(parse_i2c_address))]
        address: u8,

        /// Number of bytes to read
        #[arg(short, long)]
        count: usize,
    },

    /// Write bytes through I2C bus to device at given address
    Write {
        /// I2C slave address (7-bit, 0x00–0x7F)
        #[arg(short, long, value_parser(parse_i2c_address))]
        address: u8,

        /// Bytes to transfer
        #[arg(short, long, num_args(1..), value_parser(parse_byte))]
        bytes: Vec<u8>,
    },

    /// Write bytes follwed by read bytes
    WriteRead {
        /// I2C slave address (7-bit, 0x00–0x7F)
        #[arg(short, long, value_parser(parse_i2c_address))]
        address: u8,

        /// Bytes to transfer
        #[arg(short, long, num_args(1..), value_parser(parse_byte))]
        bytes: Vec<u8>,

        /// Number of bytes to read
        #[arg(short, long)]
        count: usize,
    },
}

#[derive(Subcommand, Debug)]
enum SpiCommands {
    /// Read bytes through SPI bus
    Read {
        /// Number of bytes to read
        #[arg(short, long)]
        count: usize,
    },

    /// Write bytes through SPI bus
    Write {
        /// Bytes to transfer
        #[arg(short, long, num_args(1..), value_parser(parse_byte))]
        bytes: Vec<u8>,
    },

    /// Full-duplex SPI transfer (simultaneous write and read)
    Transfer {
        /// Bytes to send (received data will be the same length)
        #[arg(short, long, num_args(1..), value_parser(parse_byte))]
        bytes: Vec<u8>,
    },

    /// Write bytes followed by read bytes (half-duplex)
    WriteRead {
        /// Number of bytes to read
        #[arg(short, long)]
        count: usize,

        /// Bytes to transfer
        #[arg(short, long, num_args(1..), value_parser(parse_byte))]
        bytes: Vec<u8>,
    },
}

fn print_data(data: &[u8], format: &OutputFormat) {
    match format {
        OutputFormat::Hex => {
            for (i, b) in data.iter().enumerate() {
                if i > 0 && i % 16 == 0 {
                    println!();
                }
                print!("{:02x} ", b);
            }
            println!();
        }
        OutputFormat::Binary => {
            use std::io::Write;
            std::io::stdout().write_all(data).unwrap();
        }
        OutputFormat::Ascii => {
            for (i, b) in data.iter().enumerate() {
                if i > 0 && i % 16 == 0 {
                    println!();
                }
                let ch = if b.is_ascii_graphic() || *b == b' ' {
                    *b as char
                } else {
                    '.'
                };
                print!("{ch}");
            }
            println!();
        }
    }
}

impl Cli {
    fn connect(&self) -> PicoDeGallo {
        if let Some(serial_number) = &self.serial_number {
            PicoDeGallo::new_with_serial_number(serial_number)
        } else {
            PicoDeGallo::new()
        }
    }

    pub async fn run(&self) -> Result<()> {
        match &self.command {
            Commands::List => Self::list_devices(),
            Commands::Version => self.version().await,
            Commands::I2c { command } => match command {
                I2cCommands::Scan { reserved } => self.i2c_scan(*reserved).await,
                I2cCommands::Read { address, count } => self.i2c_read(address, count).await,
                I2cCommands::Write { address, bytes } => self.i2c_write(address, bytes).await,
                I2cCommands::WriteRead { address, bytes, count } => {
                    self.i2c_write_then_read(address, bytes, count).await
                }
            },
            Commands::Spi { command } => match command {
                SpiCommands::Read { count } => self.spi_read(count).await,
                SpiCommands::Write { bytes } => self.spi_write(bytes).await,
                SpiCommands::Transfer { bytes } => self.spi_transfer(bytes).await,
                SpiCommands::WriteRead { count, bytes } => self.spi_write_then_read(bytes, count).await,
            },
            Commands::SetConfig {
                i2c_frequency,
                spi_frequency,
                spi_first_transition,
                spi_idle_low,
            } => {
                self.set_config(*i2c_frequency, *spi_frequency, *spi_first_transition, *spi_idle_low)
                    .await
            }
        }
    }

    fn list_devices() -> Result<()> {
        let devices = list_devices();
        if devices.is_empty() {
            println!("No Pico de Gallo devices found.");
            return Ok(());
        }

        for dev in &devices {
            let product = dev.product.as_deref().unwrap_or("(unknown product)");
            let serial = dev.serial_number.as_deref().unwrap_or("(unknown)");
            println!(" - {product} - {serial}");
        }
        Ok(())
    }

    async fn version(&self) -> Result<()> {
        let pg = self.connect();

        match pg.version().await {
            Ok(version) => {
                println!(
                    "Pico de Gallo FW v{}.{}.{}",
                    version.major, version.minor, version.patch
                );
                Ok(())
            }
            Err(_) => Err(eyre!("Failed to get version")),
        }
    }

    async fn i2c_scan(&self, reserved: bool) -> Result<()> {
        let pg = self.connect();

        let mut builder = Builder::with_capacity(17, 8);
        builder.push_record(
            (0..=16)
                .map(|i| if i == 0 { String::new() } else { format!("{:x}", i - 1) })
                .collect::<Vec<_>>(),
        );

        for hi in 0..=7 {
            let mut row = vec![format!("{:x} ", hi)];

            for lo in 0..=15 {
                let address = hi << 4 | lo;
                let stat = match address {
                    0x00..=0x07 | 0x78..=0x7f => {
                        if reserved {
                            match pg.i2c_read(address, 1).await {
                                Ok(_) => format!("{:02x}", address),
                                Err(_) => "--".to_string(),
                            }
                        } else {
                            "RR".to_string()
                        }
                    }
                    _ => match pg.i2c_read(address, 1).await {
                        Ok(_) => format!("{:02x}", address),
                        Err(_) => "--".to_string(),
                    },
                };

                row.push(stat);
            }

            builder.push_record(row);
        }

        let mut table = builder.build();
        table.modify(Rows::first(), Alignment::right());
        table.with(Style::rounded());

        println!("{}", table);

        Ok(())
    }

    async fn i2c_read(&self, address: &u8, count: &usize) -> Result<()> {
        let pg = self.connect();

        let buf = match pg.i2c_read(*address, *count as u16).await {
            Ok(data) => data,
            Err(e) => return Err(eyre!("{:?}", e).wrap_err("i2c_read failed")),
        };

        print_data(&buf, &self.format);

        Ok(())
    }

    async fn i2c_write(&self, address: &u8, bytes: &[u8]) -> Result<()> {
        let pg = self.connect();

        if pg.i2c_write(*address, bytes).await.is_ok() {
            Ok(())
        } else {
            Err(eyre!("i2c_write failed for address {:#04x}", address))
        }
    }

    async fn i2c_write_then_read(&self, address: &u8, bytes: &[u8], count: &usize) -> Result<()> {
        let pg = self.connect();

        let buf = match pg.i2c_write_read(*address, bytes, *count as u16).await {
            Ok(data) => data,
            Err(e) => return Err(eyre!("{:?}", e).wrap_err("i2c_write_read failed")),
        };

        print_data(&buf, &self.format);

        Ok(())
    }

    async fn spi_read(&self, count: &usize) -> Result<()> {
        let pg = self.connect();

        let buf = match pg.spi_read(*count as u16).await {
            Ok(data) => data,
            Err(e) => return Err(eyre!("{:?}", e).wrap_err("spi_read failed")),
        };

        print_data(&buf, &self.format);

        Ok(())
    }

    async fn spi_write(&self, bytes: &[u8]) -> Result<()> {
        let pg = self.connect();

        if pg.spi_write(bytes).await.is_ok() {
            Ok(())
        } else {
            Err(eyre!("spi_write failed"))
        }
    }

    async fn spi_transfer(&self, bytes: &[u8]) -> Result<()> {
        let pg = self.connect();

        let buf = pg
            .spi_transfer(bytes)
            .await
            .map_err(|e| eyre!("{:?}", e).wrap_err("spi_transfer failed"))?;

        print_data(&buf, &self.format);

        Ok(())
    }

    async fn spi_write_then_read(&self, bytes: &[u8], count: &usize) -> Result<()> {
        self.spi_write(bytes).await?;
        self.spi_read(count).await
    }

    async fn set_config(
        &self,
        i2c_frequency: u32,
        spi_frequency: u32,
        spi_first_transition: bool,
        spi_idle_low: bool,
    ) -> Result<()> {
        let pg = self.connect();

        let spi_polarity = if spi_idle_low {
            SpiPolarity::IdleLow
        } else {
            SpiPolarity::IdleHigh
        };

        let spi_phase = if spi_first_transition {
            SpiPhase::CaptureOnFirstTransition
        } else {
            SpiPhase::CaptureOnSecondTransition
        };

        if pg
            .set_config(i2c_frequency, spi_frequency, spi_phase, spi_polarity)
            .await
            .is_err()
        {
            Err(eyre!("set config failed"))
        } else {
            Ok(())
        }
    }
}

fn parse_byte(s: &str) -> Result<u8, ParseIntError> {
    if let Some(hex) = s.strip_prefix("0x") {
        u8::from_str_radix(hex, 16)
    } else if let Some(bin) = s.strip_prefix("0b") {
        u8::from_str_radix(bin, 2)
    } else {
        s.parse::<u8>()
    }
}

/// Parse an I2C 7-bit address (0x00–0x7F).
fn parse_i2c_address(s: &str) -> Result<u8, String> {
    let byte = parse_byte(s).map_err(|e| e.to_string())?;
    if byte > 0x7F {
        return Err(format!("I2C address {s} exceeds 7-bit range (max 0x7F)"));
    }
    Ok(byte)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    // ----------------------------- parse_byte tests -----------------------------

    #[test]
    fn parse_byte_decimal() {
        assert_eq!(parse_byte("0").unwrap(), 0);
        assert_eq!(parse_byte("255").unwrap(), 255);
        assert_eq!(parse_byte("42").unwrap(), 42);
    }

    #[test]
    fn parse_byte_hex() {
        assert_eq!(parse_byte("0x00").unwrap(), 0x00);
        assert_eq!(parse_byte("0xFF").unwrap(), 0xFF);
        assert_eq!(parse_byte("0x48").unwrap(), 0x48);
        assert_eq!(parse_byte("0xab").unwrap(), 0xAB);
    }

    #[test]
    fn parse_byte_binary() {
        assert_eq!(parse_byte("0b00000000").unwrap(), 0);
        assert_eq!(parse_byte("0b11111111").unwrap(), 255);
        assert_eq!(parse_byte("0b10101010").unwrap(), 0xAA);
    }

    #[test]
    fn parse_byte_overflow_fails() {
        assert!(parse_byte("256").is_err());
        assert!(parse_byte("0x100").is_err());
        assert!(parse_byte("0b100000000").is_err());
    }

    #[test]
    fn parse_byte_invalid_fails() {
        assert!(parse_byte("xyz").is_err());
        assert!(parse_byte("0xGG").is_err());
        assert!(parse_byte("0b2").is_err());
        assert!(parse_byte("").is_err());
    }

    // ----------------------------- CLI parsing tests -----------------------------

    #[test]
    fn cli_no_args_requires_help() {
        // arg_required_else_help = true means no-args should fail
        let result = Cli::try_parse_from(["gallo"]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_version_subcommand() {
        let cli = Cli::try_parse_from(["gallo", "version"]).unwrap();
        assert!(matches!(cli.command, Commands::Version));
        assert!(cli.serial_number.is_none());
    }

    #[test]
    fn cli_list_subcommand() {
        let cli = Cli::try_parse_from(["gallo", "list"]).unwrap();
        assert!(matches!(cli.command, Commands::List));
    }

    #[test]
    fn cli_version_with_serial() {
        let cli = Cli::try_parse_from(["gallo", "-s", "ABCD1234", "version"]).unwrap();
        assert_eq!(cli.serial_number.as_deref(), Some("ABCD1234"));
        assert!(matches!(cli.command, Commands::Version));
    }

    #[test]
    fn cli_i2c_read() {
        let cli = Cli::try_parse_from(["gallo", "i2c", "read", "-a", "0x48", "-c", "4"]).unwrap();
        match cli.command {
            Commands::I2c {
                command: I2cCommands::Read { address, count },
            } => {
                assert_eq!(address, 0x48);
                assert_eq!(count, 4);
            }
            _ => panic!("expected I2c Read command"),
        }
    }

    #[test]
    fn cli_i2c_write() {
        let cli = Cli::try_parse_from(["gallo", "i2c", "write", "-a", "0x50", "-b", "0xDE", "0xAD"]).unwrap();
        match cli.command {
            Commands::I2c {
                command: I2cCommands::Write { address, bytes },
            } => {
                assert_eq!(address, 0x50);
                assert_eq!(bytes, vec![0xDE, 0xAD]);
            }
            _ => panic!("expected I2c Write command"),
        }
    }

    #[test]
    fn cli_i2c_write_read() {
        let cli = Cli::try_parse_from(["gallo", "i2c", "write-read", "-a", "0x68", "-b", "0x01", "-c", "6"]).unwrap();
        match cli.command {
            Commands::I2c {
                command: I2cCommands::WriteRead { address, bytes, count },
            } => {
                assert_eq!(address, 0x68);
                assert_eq!(bytes, vec![0x01]);
                assert_eq!(count, 6);
            }
            _ => panic!("expected I2c WriteRead command"),
        }
    }

    #[test]
    fn cli_i2c_scan() {
        let cli = Cli::try_parse_from(["gallo", "i2c", "scan"]).unwrap();
        match cli.command {
            Commands::I2c {
                command: I2cCommands::Scan { reserved },
            } => {
                assert!(!reserved);
            }
            _ => panic!("expected I2c Scan command"),
        }
    }

    #[test]
    fn cli_i2c_scan_reserved() {
        let cli = Cli::try_parse_from(["gallo", "i2c", "scan", "-r"]).unwrap();
        match cli.command {
            Commands::I2c {
                command: I2cCommands::Scan { reserved },
            } => {
                assert!(reserved);
            }
            _ => panic!("expected I2c Scan command"),
        }
    }

    #[test]
    fn cli_spi_read() {
        let cli = Cli::try_parse_from(["gallo", "spi", "read", "-c", "16"]).unwrap();
        match cli.command {
            Commands::Spi {
                command: SpiCommands::Read { count },
            } => {
                assert_eq!(count, 16);
            }
            _ => panic!("expected Spi Read command"),
        }
    }

    #[test]
    fn cli_spi_write() {
        let cli = Cli::try_parse_from(["gallo", "spi", "write", "-b", "0xCA", "0xFE"]).unwrap();
        match cli.command {
            Commands::Spi {
                command: SpiCommands::Write { bytes },
            } => {
                assert_eq!(bytes, vec![0xCA, 0xFE]);
            }
            _ => panic!("expected Spi Write command"),
        }
    }

    #[test]
    fn cli_spi_transfer() {
        let cli = Cli::try_parse_from(["gallo", "spi", "transfer", "-b", "0x01", "0x02", "0x03"]).unwrap();
        match cli.command {
            Commands::Spi {
                command: SpiCommands::Transfer { bytes },
            } => {
                assert_eq!(bytes, vec![0x01, 0x02, 0x03]);
            }
            _ => panic!("expected Spi Transfer command"),
        }
    }

    #[test]
    fn cli_set_config() {
        let cli = Cli::try_parse_from([
            "gallo",
            "set-config",
            "--i2c-frequency",
            "400000",
            "--spi-frequency",
            "1000000",
            "--spi-first-transition",
            "--spi-idle-low",
        ])
        .unwrap();
        match cli.command {
            Commands::SetConfig {
                i2c_frequency,
                spi_frequency,
                spi_first_transition,
                spi_idle_low,
            } => {
                assert_eq!(i2c_frequency, 400_000);
                assert_eq!(spi_frequency, 1_000_000);
                assert!(spi_first_transition);
                assert!(spi_idle_low);
            }
            _ => panic!("expected SetConfig command"),
        }
    }

    #[test]
    fn cli_set_config_defaults() {
        let cli = Cli::try_parse_from([
            "gallo",
            "set-config",
            "--i2c-frequency",
            "100000",
            "--spi-frequency",
            "500000",
        ])
        .unwrap();
        match cli.command {
            Commands::SetConfig {
                spi_first_transition,
                spi_idle_low,
                ..
            } => {
                assert!(!spi_first_transition);
                assert!(!spi_idle_low);
            }
            _ => panic!("expected SetConfig command"),
        }
    }

    #[test]
    fn cli_set_config_missing_required_fails() {
        // Missing --spi-frequency
        let result = Cli::try_parse_from(["gallo", "set-config", "--i2c-frequency", "400000"]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_i2c_read_missing_address_fails() {
        let result = Cli::try_parse_from(["gallo", "i2c", "read", "-c", "4"]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_unknown_subcommand_fails() {
        let result = Cli::try_parse_from(["gallo", "uart"]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_i2c_without_subcommand_fails() {
        let result = Cli::try_parse_from(["gallo", "i2c"]);
        assert!(result.is_err());
    }

    #[test]
    fn cli_spi_without_subcommand_fails() {
        let result = Cli::try_parse_from(["gallo", "spi"]);
        assert!(result.is_err());
    }
}
