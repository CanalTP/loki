// Copyright  (C) 2020, Kisio Digital and/or its affiliates. All rights reserved.
//
// This file is part of Navitia,
// the software to build cool stuff with public transport.
//
// Hope you'll enjoy and contribute to this project,
// powered by Kisio Digital (www.kisio.com).
// Help us simplify mobility and open public transport:
// a non ending quest to the responsive locomotion way of traveling!
//
// This contribution is a part of the research and development work of the
// IVA Project which aims to enhance traveler information and is carried out
// under the leadership of the Technological Research Institute SystemX,
// with the partnership and support of the transport organization authority
// Ile-De-France Mobilités (IDFM), SNCF, and public funds
// under the scope of the French Program "Investissements d’Avenir".
//
// LICENCE: This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.
//
// Stay tuned using
// twitter @navitia
// channel `#navitia` on riot https://riot.im/app/#/room/#navitia:matrix.org
// https://groups.google.com/d/forum/navitia
// www.navitia.io

pub mod transit_realtime_proto {
    include!(concat!(env!("OUT_DIR"), "/transit_realtime.rs"));
}

use failure::{format_err, Error};
use lapin::{options::*, types::FieldTable, Channel, Connection, ConnectionProperties, Queue};
use launch::loki::tracing::{error, info};
use launch::realtime::{handle_realtime, RealTimeMessage};
use prost::Message;
use serde::{Deserialize, Serialize};
use structopt::StructOpt;

pub fn default_host() -> String {
    "localhost".to_string()
}
pub fn default_username() -> String {
    "guest".to_string()
}
pub fn default_password() -> String {
    "guest".to_string()
}
pub fn default_vhost() -> String {
    "/".to_string()
}
pub fn default_exchange() -> String {
    "navitia".to_string()
}
pub fn default_port() -> u16 {
    5672
}
pub fn default_rt_topics() -> Vec<String> {
    Vec::new()
}
pub fn default_queue_auto_delete() -> bool {
    true
}

#[derive(Debug, Serialize, Deserialize, StructOpt, Clone)]
#[structopt(rename_all = "snake_case")]
pub struct BrockerConfig {
    #[structopt(long, default_value = "default_host")]
    #[serde(default = "default_host")]
    pub host: String,

    #[structopt(long, default_value = "default_port")]
    #[serde(default = "default_port")]
    pub port: u16,

    #[structopt(long, default_value = "default_username")]
    #[serde(default = "default_username")]
    pub username: String,

    #[structopt(long, default_value = "default_password")]
    #[serde(default = "default_password")]
    pub password: String,

    #[structopt(long, default_value = "default_vhost")]
    #[serde(default = "default_vhost")]
    pub vhost: String,

    #[structopt(long, default_value = "default_exchange")]
    #[serde(default = "default_exchange")]
    pub exchange: String,

    #[structopt(long, default_value = "default_rt_topics")]
    #[serde(default = "default_rt_topics")]
    pub rt_topics: Vec<String>,

    #[structopt(long)]
    #[serde(default = "default_queue_auto_delete")]
    pub queue_auto_delete: bool,
}

pub struct RealTimeWorker<'config> {
    pub config: &'config BrockerConfig,
    connection: Connection,
    channel: Channel,
    queue_task: Queue,
    queue_rt: Queue,
}

impl<'config> RealTimeWorker<'config> {
    pub fn new(config: &'config BrockerConfig) -> Result<Self, Error> {
        let connection = RealTimeWorker::create_connection(config)?;
        let channel = RealTimeWorker::create_channel(&connection)?;
        let queue_task =
            RealTimeWorker::declare_queue(&channel, format!("{}_task", "loki_hostname").as_str())?;
        let queue_rt =
            RealTimeWorker::declare_queue(&channel, format!("{}_rt", "loki_hostname").as_str())?;
        Ok(Self {
            config,
            connection,
            channel,
            queue_task,
            queue_rt,
        })
    }

    fn create_connection(config: &BrockerConfig) -> Result<Connection, Error> {
        let address = format!(
            "amqp://{}:{}@{}:{}{}",
            config.username, config.password, config.host, config.port, config.vhost
        );
        info!("Connection to rabbitmq {}", address);
        let connection =
            Connection::connect(address.as_str(), ConnectionProperties::default()).wait()?;
        info!("connected to rabbitmq {} successfully", address);
        Ok(connection)
    }

    fn create_channel(connection: &Connection) -> Result<Channel, Error> {
        let channel = connection.create_channel().wait()?;
        info!("channel created successfully");
        Ok(channel)
    }

    fn declare_queue(channel: &Channel, queue_name: &str) -> Result<Queue, Error> {
        let queue = channel
            .queue_declare(
                queue_name,
                QueueDeclareOptions::default(),
                FieldTable::default(),
            )
            .wait()?;
        Ok(queue)
    }

    async fn consume(&self) {
        let consumer = self
            .channel
            .basic_consume(
                self.queue_rt.name().as_str(),
                "my_consumer",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await
            .expect("basic_consume");

        for delivery in consumer {
            info!("received message: {:?}", delivery);
            if let Ok((_, delivery)) = delivery {
                delivery
                    .ack(BasicAckOptions::default())
                    .await
                    .expect("basic_ack");

                println!("{:?}", delivery);

                let proto_message = decode_amqp_message(&delivery);
                match proto_message {
                    Ok(proto_message) => {
                        // map to RealTimeMessage
                        let rt_message = RealTimeMessage::default();
                        handle_realtime(&rt_message);
                    }
                    Err(err) => {
                        error!("{}", err.to_string())
                    }
                }
            }
        }
    }

    pub async fn listen(&self) {
        loop {
            self.consume().await;
        }
    }
}

fn decode_amqp_message(
    message: &lapin::message::Delivery,
) -> Result<transit_realtime_proto::FeedMessage, Error> {
    let payload = &message.data;
    transit_realtime_proto::FeedMessage::decode(&payload[..])
        .map_err(|err| format_err!("Could not decode zmq message into protobuf: \n {}", err))
}
