use crate::selection::SelectionType;

use super::GeneticRenewalParam;

const DEFAULT_MUTATION_RATE: f32 = 0.01;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct GeneticPool {
    pub count: usize,
    pub mutation_rate: f32,
    pub selection_type: SelectionType,
}

impl GeneticPool {
    pub fn from_params(params: &GeneticRenewalParam, total: usize) -> Self {
        GeneticPool {
            count: (params.ratio * total as f32) as usize,
            mutation_rate: params.mutation_rate.unwrap_or(DEFAULT_MUTATION_RATE),
            selection_type: params.selection_type,
        }
    }

    pub fn from_optional_params(params: Option<&GeneticRenewalParam>, total: usize) -> Self {
        match params {
            Some(p) => Self::from_params(p, total),
            None => Self::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        evolution::{genetic_pool::DEFAULT_MUTATION_RATE, GeneticRenewalParam},
        selection::SelectionType,
    };

    use super::GeneticPool;

    #[test]
    fn test_genetic_pool_from_param_should_use_default_rate_when_required() {
        // Given
        let params = GeneticRenewalParam {
            mutation_rate: None,
            ratio: 0.1,
            selection_type: SelectionType::Chance,
        };

        // When
        let result = GeneticPool::from_params(&params, 10);

        // Then
        assert_eq!(DEFAULT_MUTATION_RATE, result.mutation_rate);
    }

    #[test]
    fn test_genetic_pool_from_param_should_return_valid_count() {
        // Given
        let total = 10;
        let params = GeneticRenewalParam {
            mutation_rate: None,
            ratio: 0.1,
            selection_type: SelectionType::Chance,
        };

        // When
        let result = GeneticPool::from_params(&params, total);

        // Then
        assert_eq!((total as f32 * params.ratio) as usize, result.count);
    }

    #[test]
    fn test_genetic_pool_from_param_should_use_param_selection_type() {
        // Given
        let params = GeneticRenewalParam {
            mutation_rate: None,
            ratio: 0.1,
            selection_type: SelectionType::Chance,
        };

        // When
        let result = GeneticPool::from_params(&params, 10);

        // Then
        assert_eq!(params.selection_type, result.selection_type);
    }

    #[test]
    fn test_genetic_pool_from_optional_param_should_return_valid_pool_when_some() {
        // Given
        let total = 10;
        let params = GeneticRenewalParam {
            mutation_rate: Some(0.1),
            ratio: 5.0,
            selection_type: SelectionType::Ranking(8),
        };

        // When
        let result = GeneticPool::from_optional_params(Some(&params), total);

        // Then
        assert_eq!(GeneticPool::from_params(&params, total), result);
    }

    #[test]
    fn test_genetic_pool_from_optional_param_should_return_default_when_none() {
        // When
        let result = GeneticPool::from_optional_params(None, 10);

        // Then
        assert_eq!(GeneticPool::default(), result);
    }
}
