use async_graphql::*;

/// Algorithms supported by barriers
#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum BarrierAlgorithm {
    /// Algorithm with impulse for open and close at the same pin
    OpenClose,

    /// Algorithm with one impulse only for open
    Open,

    /// Algorithm with impulse for open and close at two different pins
    TwoDoors,
}

#[derive(SimpleObject)]
#[graphql(complex)]
pub struct BarrierModel {
  id: ID,
  name: String,
  algorithm: BarrierAlgorithm,
  created_at: i64,
  updated_at: Option<i64>,

  #[graphql(skip)]
  manufacturer_id: Option<String>,
}
