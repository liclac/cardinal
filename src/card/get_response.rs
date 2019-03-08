use crate::cmd::{Request, Response};
use std::marker::PhantomData;

pub struct GetResponse<RT: Response> {
    pub cla: u8,
    pub le: u8,

    pub _phantom_rt: PhantomData<RT>,
}

impl<RT: Response> GetResponse<RT> {
    pub fn new(cla: u8, le: u8) -> Self {
        Self {
            cla,
            le,
            _phantom_rt: PhantomData {},
        }
    }
}

impl<RT: Response> Request for GetResponse<RT> {
    type Returns = RT;

    fn cla(&self) -> u8 {
        self.cla
    }
    fn ins(&self) -> u8 {
        0xC0
    }
    fn le(&self) -> Option<usize> {
        Some(self.le as usize)
    }
}
