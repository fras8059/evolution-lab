mod evolution_engine;
mod genetic_pool;

pub use evolution_engine::EvolutionEngine;
use strum::{Display, EnumIter};
use thiserror::Error;
use validator::{Validate, ValidationError, ValidationErrors};

use crate::{selection::SelectionType, Evaluation};

#[derive(Copy, Clone, Debug, PartialEq, Default, EnumIter, Display)]
pub enum EvolutionStatus {
    #[default]
    New,
    Initializing,
    Running,
    Halting,
    Halted,
    Completed,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EventType {
    Evaluated,
    GenerationCreated,
    StatusChanged(EvolutionStatus),
}

#[derive(Clone, Debug, Validate)]
pub struct GeneticRenewalParam {
    #[validate(range(min = 0f32, max = 1f32))]
    pub mutation_rate: Option<f32>,
    #[validate(range(min = 0f32, max = 1f32))]
    pub ratio: f32,
    pub selection_type: SelectionType,
}

#[derive(Clone, Debug, Validate)]
#[validate(schema(function = "validate_generation_renewal_config"))]
pub struct GenerationRenewalConfig {
    #[validate(nested)]
    pub cloning: Option<GeneticRenewalParam>,
    #[validate(nested)]
    pub crossover: Option<GeneticRenewalParam>,
}

#[derive(Clone, Debug, Validate)]
pub struct EvolutionConfig {
    #[validate(range(min = 1))]
    pub population_size: usize,
    #[validate(nested)]
    pub generation_renewal_config: Option<GenerationRenewalConfig>,
}

#[derive(Error, Debug, PartialEq)]
pub enum EvolutionError {
    #[error("Invalid selection: {0}")]
    InvalidSelection(String),
    #[error("Settings are not valid: {0}")]
    InvalidSettings(ValidationErrors),
    #[error("Unable to run evolution from status: {0}")]
    InvalidStatus(EvolutionStatus),
    #[error("Lock error: {0}")]
    Lock(String),
}

pub type EvolutionResult<State> = Result<Snapshot<State>, EvolutionError>;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Snapshot<G> {
    pub generation: u64,
    pub evaluations: Vec<Evaluation<G>>,
}

fn validate_generation_renewal_config(
    config: &GenerationRenewalConfig,
) -> Result<(), ValidationError> {
    if let Some((cloning_param, crossover_param)) =
        config.cloning.as_ref().zip(config.crossover.as_ref())
    {
        if cloning_param.ratio + crossover_param.ratio >= 1.0 {
            return Err(ValidationError::new("excessive_rates"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::selection::SelectionType;

    use super::{
        validate_generation_renewal_config, GenerationRenewalConfig, GeneticRenewalParam, Snapshot,
    };

    #[test]
    fn test_snapshot_default() {
        let result = Snapshot::<u8>::default();

        assert_eq!(
            Snapshot {
                evaluations: vec![],
                generation: 0
            },
            result
        );
    }

    #[test]
    fn test_validate_generation_renewal_config() {
        // Given
        let wrong_config = GenerationRenewalConfig {
            cloning: Some(GeneticRenewalParam {
                mutation_rate: None,
                ratio: 0.51,
                selection_type: SelectionType::Chance,
            }),
            crossover: Some(GeneticRenewalParam {
                mutation_rate: None,
                ratio: 0.51,
                selection_type: SelectionType::Chance,
            }),
        };

        // When
        let result = validate_generation_renewal_config(&wrong_config);

        // Then
        assert!(
            matches!(result, Err(_)),
            "Should return err when cumulated ratios are greater than 1.0"
        );

        // Given
        let right_config = GenerationRenewalConfig {
            cloning: None,
            crossover: None,
        };

        // When
        let result = validate_generation_renewal_config(&right_config);

        // Then
        assert!(
            matches!(result, Ok(())),
            "Should return Ok for valid config"
        );
    }
}
