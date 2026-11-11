//! 门牌命令：零校验激活 / 状态 / 复位。

use crate::api::Api;
use crate::infra::source::{self, GateState};
use tauri::State;

#[tauri::command]
pub async fn gate_open(state: State<'_, Api>, address: String) -> Result<GateState, String> {
    let previous = source::base();
    let next = source::gate_open(&address)?;
    if previous != next.base {
        state.session.clear();
    }
    Ok(next)
}

#[tauri::command]
pub async fn gate_state() -> Result<GateState, String> {
    Ok(source::gate_state())
}

#[tauri::command]
pub async fn gate_reset(state: State<'_, Api>) -> Result<GateState, String> {
    let next = source::gate_reset();
    state.session.clear();
    Ok(next)
}
