//! System metrics: samples CPU, memory, swap, disk, network and temperature on a timer and
//! publishes each sample to the signal hub for modules subscribed to `metrics`.
pub mod adapters;
pub mod domain;
