use sed_packet::Uid;

pub trait MethodParam {
    const METHOD_ID: Uid;

    fn method_id(&self) -> Uid {
        Self::METHOD_ID
    }
}
