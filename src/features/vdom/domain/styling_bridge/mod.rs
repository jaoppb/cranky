mod mapper;
mod pseudo;
mod resolver;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_interaction;

pub use pseudo::compute_pseudo_classes;
