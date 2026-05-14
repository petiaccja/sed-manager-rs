//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use core::task::Poll::*;

use crate::rpc::args::UnwrapMethodArgs;
use crate::rpc::protocol::shared::pipe::{SinkPipe, SourcePipe};
use crate::rpc::{Error, PackagedMethod, SessionIdentifier};
use crate::spec::sm_method_id;

pub type Input = Result<PackagedMethod, Error>;
pub type Output = Result<PackagedMethod, Error>;

pub fn filter_close_session(
    input: &mut dyn SourcePipe<Input>,
    output: &mut dyn SinkPipe<Output>,
    aborted: &mut dyn SinkPipe<SessionIdentifier>,
) {
    while let Ready(Some(method)) = input.pop() {
        if let Ok(PackagedMethod::Call(call)) = method {
            if call.method_id == sm_method_id::CLOSE_SESSION {
                if let Ok((hsn, tsn)) = call.args.unwrap_method_args() {
                    aborted.push(SessionIdentifier { hsn, tsn });
                }
            } else {
                output.push(Ok(PackagedMethod::Call(call)));
            }
        } else {
            output.push(method);
        }
    }
    if input.is_done() {
        output.close();
        aborted.close();
    }
}
