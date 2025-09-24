use crate::{
    ProbeError,
    register::DriverRegisterData,
};

pub(crate) fn probe_with<'a>(
    registers: impl Iterator<Item = &'a DriverRegisterData>,
    stop_if_fail: bool,
) -> Result<(), ProbeError> {
    for one in registers {
        match probe_one(one) {
            Ok(_) => {} // Successfully probed, move to the next
            Err(e) => {
                if stop_if_fail {
                    return Err(e);
                } else {
                    warn!("Probe failed for [{}]: {}", one.register.name, e);
                }
            }
        }
    }

    Ok(())
}

fn probe_one(_one: &DriverRegisterData) -> Result<(), ProbeError> {


    // handle_probe_one_result(one.id, )?;
    Ok(())
}
