# RDrive - Rust Dynamic Driver Framework

A dynamic driver management framework for embedded systems written in Rust.

## Architecture Overview

### rdif-* Interface Crates

Hardware abstraction interface crates with `Any` for dynamic dispatch:

- `rdif-base`: Core driver traits and error types
- `rdif-intc`: Interrupt controller interface
- `rdif-clk`: Clock management interface  
- `rdif-serial`: Serial communication interface
- `rdif-timer`: Timer/counter interface
- `rdif-block`: Block device interface
- `rdif-power`: Power management interface
- `rdif-systick`: System tick interface
- `rdif-net`: Network interface

### rdrive-macros

Procedural macros to simplify driver registration and module generation.

### rdrive Core

The main driver container responsible for:

- Driver registration and discovery
- Device probing and initialization
- Device lifecycle management
- Type-safe device access

## Device Ownership Model

Devices are managed through a long-term borrowing model using `Device<T>` wrappers. Unlike `Mutex<T>`, when a task borrows a device, it gains exclusive ownership. Other tasks attempting to borrow will receive an error with the owner's task ID, enabling forced task termination if needed.

Key features:

- Lock-free operations once ownership is acquired
- Weak pointer support for interrupt handlers
- Cloning `Device<T>` creates weak references for fast indexing

## Driver Registration

Drivers can be registered from different crates using the `module_driver!` macro:

```rust
use rdrive::{
    module_driver,
    register::{ProbeKind, ProbeLevel, ProbePriority, FdtInfo},
    probe::OnProbeError,
    PlatformDevice,
    DriverGeneric,
};

struct GicV3Driver {
    // Driver implementation
}

impl DriverGeneric for GicV3Driver {
    fn open(&mut self) -> Result<(), rdrive::KError> {
        // Initialize hardware
        Ok(())
    }

    fn close(&mut self) -> Result<(), rdrive::KError> {
        // Cleanup hardware
        Ok(())
    }
}

impl rdrive::driver::intc::Interface for GicV3Driver {
    // Implement interrupt controller interface
    fn enable_irq(&mut self, irq: rdrive::IrqId) -> Result<(), rdrive::driver::intc::IntcError> {
        // Enable interrupt implementation
        Ok(())
    }
    
    // ... other required methods
}

fn probe_gicv3(fdt: FdtInfo<'_>, dev: PlatformDevice) -> Result<(), OnProbeError> {
    let node = fdt.node;
    let mut reg = node.reg().ok_or("No reg property")?;
    
    let gicd_reg = reg.next().ok_or("Missing GICD register")?;
    let gicr_reg = reg.next().ok_or("Missing GICR register")?;
    
    let driver = GicV3Driver::new(
        gicd_reg.address as usize,
        gicr_reg.address as usize,
    );
    
    dev.register(driver);
    Ok(())
}

module_driver! {
    name: "GICv3",
    level: ProbeLevel::PreKernel,
    priority: ProbePriority::INTC,
    probe_kinds: &[ProbeKind::Fdt {
        compatibles: &["arm,gic-v3"],
        on_probe: probe_gicv3,
    }],
}
```

## System Integration

### 1. Linker Script Configuration

Add the following section to your linker script:

```ld
.driver.register : ALIGN(4K) {
    _sdriver = .;
    *(.driver.register .driver.register.*)
    _edriver = .;
    . = ALIGN(4K);
}
```

### 2. Driver Registration Discovery

```rust
use rdrive::register::DriverRegisterSlice;

fn driver_registers() -> &'static [u8] {
    unsafe extern "C" {
        fn _sdriver();
        fn _edriver();
    }

    unsafe { 
        core::slice::from_raw_parts(
            _sdriver as *const u8, 
            _edriver as usize - _sdriver as usize
        ) 
    }
}

fn get_driver_registers() -> DriverRegisterSlice {
    DriverRegisterSlice::from_raw(driver_registers())
}
```

### 3. System Initialization

```rust
use rdrive::{Platform, probe_pre_kernel, probe_all};
use core::ptr::NonNull;

fn init_drivers() {
    // Initialize with device tree
    let fdt_addr = /* device tree address */;
    let platform = Platform::Fdt {
        addr: NonNull::new(fdt_addr).unwrap(),
    };

    // Initialize the driver framework
    rdrive::init(platform).unwrap();

    // Register all discovered drivers
    rdrive::register_append(get_driver_registers().as_slice());

    // Probe critical drivers first (interrupt controllers, etc.)
    rdrive::probe_pre_kernel().unwrap();

    // Initialize interrupt system
    // irq::init_main_cpu();

    // Probe remaining drivers
    rdrive::probe_all(false).unwrap(); // false = don't stop on failures
}
```

### 4. Device Access

```rust
use rdrive::{get_list, get_one, driver::Intc};

fn use_devices() {
    // Get all interrupt controllers
    let intc_list = get_list::<Intc>();
    for intc in intc_list {
        println!("Found INTC: {:?}", intc.descriptor());
    }

    // Get the first available interrupt controller
    if let Some(intc) = get_one::<Intc>() {
        // Use the device - this gives exclusive access
        match intc.try_lock() {
            Ok(mut locked_intc) => {
                // Safe to use locked_intc without additional synchronization
                locked_intc.enable_irq(42.into()).unwrap();
            }
            Err(owner_id) => {
                println!("Device busy, owned by task: {:?}", owner_id);
            }
        }
    }
}
```

## Supported Probe Methods

- **Device Tree (FDT)**: Automatic device discovery from device tree
- **Static Configuration**: Manual device registration (planned)

## Examples

See the `examples/` directory for complete usage examples:

- `examples/enumerate/`: Basic driver enumeration and device tree parsing

## References

For complete integration examples, see:

- [Example](https://github.com/drivercraft/rdrive/blob/4d86d7233ce7f55968d225a4048f4a8e0487c377/examples/enumerate/src/main.rs)
