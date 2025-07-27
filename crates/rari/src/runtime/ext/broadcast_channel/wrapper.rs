use crate::{error::Error, runtime::JsExecutionRuntime as Runtime};
use deno_broadcast_channel::BroadcastChannel;
use serde::{Serialize, de::DeserializeOwned};
use std::time::Duration;

#[macro_export]
macro_rules! big_json_args {
    ($($arg:expr),*) => {
        &vec![
            $(serde_json::Value::from($arg)),*
        ]
    };
}

#[allow(unused)]
pub struct BroadcastChannelWrapper<Channel: BroadcastChannel> {
    channel: Channel,
    resource: <Channel as BroadcastChannel>::Resource,
    name: String,
}
impl<Channel: BroadcastChannel> BroadcastChannelWrapper<Channel> {
    #[allow(unused)]
    pub fn new(channel: &Channel, name: impl ToString) -> Result<Self, Error> {
        let channel = channel.clone();
        let resource = channel.subscribe()?;
        let name = name.to_string();
        Ok(Self { channel, resource, name })
    }

    #[allow(unused)]
    pub fn send_sync<T: Serialize>(&self, runtime: &mut Runtime, data: T) -> Result<(), Error> {
        let tokio_rt = runtime.tokio_runtime();
        tokio_rt.block_on(self.send(runtime, data))
    }

    #[allow(unused)]
    pub async fn send<T: Serialize>(&self, runtime: &mut Runtime, data: T) -> Result<(), Error> {
        let data: Vec<u8> = runtime.call_function_async("broadcast_serialize", &data).await?;
        self.channel.send(&self.resource, self.name.clone(), data).await?;
        Ok(())
    }

    #[allow(unused)]
    pub async fn recv<T: DeserializeOwned>(
        &self,
        runtime: &mut Runtime,
        timeout: Option<Duration>,
    ) -> Result<Option<T>, Error> {
        let msg = if let Some(timeout) = timeout {
            tokio::select! {
                msg = self.channel.recv(&self.resource) => msg,
                () = tokio::time::sleep(timeout) => Ok(None),
            }
        } else {
            self.channel.recv(&self.resource).await
        }?;

        let Some((name, data)) = msg else {
            return Ok(None);
        };

        if name == self.name {
            let data: T =
                runtime.call_function_async("broadcast_deserialize", big_json_args!(data)).await?;
            Ok(Some(data))
        } else {
            Ok(None)
        }
    }

    #[allow(unused)]
    pub fn recv_sync<T: DeserializeOwned>(
        &self,
        runtime: &mut Runtime,
        timeout: Option<Duration>,
    ) -> Result<Option<T>, Error> {
        let tokio_rt = runtime.tokio_runtime();
        tokio_rt.block_on(self.recv(runtime, timeout))
    }
}

impl<Channel: BroadcastChannel> Drop for BroadcastChannelWrapper<Channel> {
    fn drop(&mut self) {
        self.channel.unsubscribe(&self.resource).ok();
    }
}
