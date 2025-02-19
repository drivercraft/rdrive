# RDrive Rust 动态驱动组件

## 架构设计

### rdif-* 接口

硬件通用操作接口，为支持动态分发，接口均设计为`object safe`。

### rdrive-macros

便于驱动注册的宏。

### rdrive 动态驱动容器

驱动容器，负责驱动注册、遍历、所有权控制。

设备所有权设计为长期借用模式，通过 `Device<T>` 进行包装，不同于 `Mutex<T>`，当一个任务借用了设备后，会获取其所有权，其他任务在尝试借用时会得到 `Error` 并得到拥有者的任务ID，可用于强制关闭任务。借用者获取所有权后，可进行无锁操作。对 `Device<T>` 的复制会获取其弱指针，方便其他任务进行快速索引，在中断中可使用弱指针强制获取所有权。

驱动可在不同 `crate` 进行注册，可实现在一个单独文件中完成所有注册流程，例如：

```rust
module_driver!(
    name: "GICv3",
    probe_kinds: &[
        ProbeKind::Fdt {
            compatibles: &["arm,gic-v3"],
            on_probe: OnProbeKindFdt::Intc(probe_gic)
        }
    ]
);

fn probe_gic(node: Node<'_>) -> Result<FdtProbeInfo, Box<dyn Error>> {
    let mut reg = node.reg().ok_or(format!("[{}] has no reg", node.name))?;

    let gicd_reg = reg.next().unwrap();
    let gicr_reg = reg.next().unwrap();
    let gicd = iomap(
        (gicd_reg.address as usize).into(),
        gicd_reg.size.unwrap_or(0x1000),
    );
    let gicr = iomap(
        (gicr_reg.address as usize).into(),
        gicr_reg.size.unwrap_or(0x1000),
    );

    Ok(FdtProbeInfo {
        hardware: Box::new(Gic::new(gicd, gicr, Default::default())),
        fdt_parse_config_fn: fdt_parse_irq_config,
    })
}
```

上述代码完成了 `Gic` 驱动注册，在后续 `probe` 阶段会调用 `probe_gic` 函数进行驱动创建。

支持通过设备树遍历设备、静态配置设备。

## 系统适配

1. `LinkerScript` 添加：

    ```ld
    .driver.register : ALIGN(4K) {
        _sdriver = .;
        *(.driver.register .driver.register.*)
        _edriver = .;
        . = ALIGN(4K);
    }
    ```

2. 系统代码中添加：

    ```rust
    fn driver_registers() -> &'static [u8] {
        unsafe extern "C" {
            fn _sdriver();
            fn _edriver();
        }

        unsafe { &*slice_from_raw_parts(
            _sdriver as *const u8, 
            _edriver as usize - _sdriver as usize) }
    }

    fn driver_registers() -> DriverRegisterSlice {
        DriverRegisterSlice::from_raw(driver_registers())
    }

    ```

3. 初始化(设备树方式)：

    ```rust
    fn init() {
        let info = DriverInfoKind::Fdt {
            addr: // 设备树地址,
        };

        // 库初始化
        rdrive::init(info);

        // 添加驱动注册器
        rdrive::register_append(driver_registers().as_slice());

        // 其他设备依赖中断控制器，优先初始化
        rdrive::probe_intc().unwrap();

        irq::init_main_cpu();

        // 任务等功能依赖系统时钟，优先初始化
        rdrive::probe_timer().unwrap();

        time::init_current_cpu();
    }
    ```

4. 遍历设备：

    ```rust
    rdrive::probe().unwrap();
    ```

5. 获取设备：

    ```rust
    let mut ls = rdrive::read(|m| m.timer.all());
    let (_, timer) = ls.pop()?;

    let mut timer = timer.upgrade()?.spin_try_borrow_by(0.into());

    // 可自由使用 mut timer，此时其他任务尝试借用会返回 Error
    ```

可参照样例：

[初始化](https://github.com/qclic/sparreal-os/blob/main/crates/sparreal-kernel/src/driver/mod.rs)

[驱动注册](https://github.com/qclic/sparreal-os/blob/main/crates/sparreal-rt/src/arch/aarch64/gic/gic_v3.rs)

[设备获取](https://github.com/qclic/sparreal-os/blob/main/crates/sparreal-kernel/src/time/mod.rs)
