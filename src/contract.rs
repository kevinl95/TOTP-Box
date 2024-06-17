use std::cmp::max;
use std::time::SystemTime;
use cosmwasm_std::{
    entry_point, to_binary, Deps, DepsMut, Env,
    MessageInfo, QueryResponse, Response, StdError, StdResult
};

use crate::errors::{CustomContractError};
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, RicherResponse};
use crate::state::{config, config_read, ContractState, Millionaire, State};

use totp_rs::{Algorithm, TOTP, Secret};


#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> StdResult<Response> {

    let state = State::default();
    config(deps.storage).save(&state)?;

    Ok(Response::default())
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, CustomContractError> {
    match msg {
        ExecuteMsg::SubmitSecret { name, secret } => try_submit_secret(deps, name, secret),
        ExecuteMsg::Reset {  } => try_reset(deps),
    }
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<QueryResponse> {
    match msg {
        QueryMsg::GetToken {} => to_binary(&query_get_totp(deps)?),
    }
}

pub fn try_submit_secret(
    deps: DepsMut,
    name: String,
    secret: String
) -> Result<Response, CustomContractError> {
    let mut state = config(deps.storage).load()?;

    match state.state {
        ContractState::Init => {
            state.service = Service::new(name, secret);
            state.state = ContractState::Done;
        }
        ContractState::Done => {
            return Err(CustomContractError::AlreadyAddedSecret);
        }
    }

    config(deps.storage).save(&state)?;

    Ok(Response::new())
}

pub fn try_reset(
    deps: DepsMut,
) -> Result<Response, CustomContractError> {
    let mut state = config(deps.storage).load()?;

    state.state = ContractState::Init;
    config(deps.storage).save(&state)?;

    Ok(Response::new()
        .add_attribute("action", "reset state"))
}

fn query_get_totp(deps: Deps) -> StdResult<TOTPResponse> {
    let state = config_read(deps.storage).load()?;

    if state.state != ContractState::Done {
        return Err(StdError::generic_err("Can't compute a token until the secret is loaded!"));
    }

    let totp = TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        Secret::Raw(state.service.secret.as_bytes().to_vec()).to_bytes().unwrap(),
    ).unwrap();

    let resp = TOTPResponse {
        token: totp.generate_current().unwrap();
    };

    Ok(resp)
}

#[cfg(test)]
mod tests {
    use super::*;

    use cosmwasm_std::testing::{mock_env, mock_info, mock_dependencies};
    use cosmwasm_std::coins;

    #[test]
    fn proper_instantialization() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let _ = query_get_totp(deps.as_ref()).unwrap_err();
    }

    #[test]
    fn get_totp() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(2, "token"));
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let msg_service = ExecuteMsg::SubmitSecret {secret: "TestSecretSuperSecret", name: "Gmail".to_string()};

        let info = mock_info("creator", &[]);

        let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg_service).unwrap();

        // it worked, let's query the state
        let value = query_get_totp(deps.as_ref()).unwrap();

        assert_eq!(value.token.len(), 6)

    }

    #[test]
    fn test_reset_state() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(2, "token"));
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let msg_service = ExecuteMsg::SubmitSecret {secret: "TestSecretSuperSecret", name: "Gmail".to_string()};

        let info = mock_info("creator", &[]);
        let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg_service).unwrap();

        // it worked, let's query the state
        let value1 = query_get_totp(deps.as_ref()).unwrap();

        let reset_msg = ExecuteMsg::Reset {};
        let _res = execute(deps.as_mut(), mock_env(), info.clone(), reset_msg).unwrap();

        let msg_service2 = ExecuteMsg::SubmitSecret {secret: "TestSecretSuperSecret", name: "Gmail".to_string()};

        let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg_service2).unwrap();

        // it worked, let's query the state
        let value2 = query_get_totp(deps.as_ref()).unwrap();

        assert_ne!(&value1.token, @value2.token)    }
}
