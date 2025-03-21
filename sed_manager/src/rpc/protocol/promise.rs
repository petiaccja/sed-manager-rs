//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use tokio::sync::oneshot;

pub struct Promise<Request, Response, Error> {
    request: Request,
    senders: Vec<oneshot::Sender<Result<Response, Error>>>,
}

impl<Request, Response, Error> Promise<Request, Response, Error> {
    pub fn new(request: Request, senders: Vec<oneshot::Sender<Result<Response, Error>>>) -> Self {
        Self { request, senders }
    }

    pub fn from_message(request: Request) -> Self {
        Self { request, senders: vec![] }
    }

    pub fn detach(self) -> (Request, Vec<oneshot::Sender<Result<Response, Error>>>) {
        (self.request, self.senders)
    }

    pub fn map<NewRequest>(self, f: impl FnOnce(Request) -> NewRequest) -> Promise<NewRequest, Response, Error> {
        Promise { request: f(self.request), senders: self.senders }
    }

    pub fn try_map<NewRequest, F>(self, f: F) -> Option<Promise<NewRequest, Response, Error>>
    where
        Error: Clone,
        F: FnOnce(Request) -> Result<NewRequest, Error>,
    {
        let (request, senders) = self.detach();
        match f(request) {
            Ok(new_request) => Some(Promise { request: new_request, senders }),
            Err(error) => {
                Promise { request: (), senders }.close_with_error(error);
                None
            }
        }
    }

    pub fn close_with_value(self, value: Response)
    where
        Response: Clone,
    {
        for sender in self.senders {
            let _ = sender.send(Ok(value.clone()));
        }
    }

    pub fn close_with_error(self, error: Error)
    where
        Error: Clone,
    {
        for sender in self.senders {
            let _ = sender.send(Err(error.clone()));
        }
    }
}
