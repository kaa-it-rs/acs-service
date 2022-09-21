use async_graphql::*;

mod auth;
pub(crate) mod barrier_manufacturer;
pub(crate) mod barrier_model;
mod error;
mod opener;
pub(crate) mod role;
pub(crate) mod simple_broker;
pub(crate) mod user;

pub(crate) use error::Error;

pub(crate) use opener::{
    CommandStatus, CommandType, OpenerCommandResult, OpenerConnectionChanged, OpenerError,
};

#[derive(MergedObject, Default)]
pub struct Mutation(auth::AuthMutation, opener::OpenerMutation);

#[derive(MergedObject, Default)]
pub struct Query(
    role::RoleQuery,
    user::UserQuery,
    opener::OpenerQuery,
    barrier_model::BarrierModelQuery,
    barrier_manufacturer::BarrierManufacturerQuery,
);

#[derive(MergedSubscription, Default)]
pub struct Subscription(opener::OpenerSubscription);
