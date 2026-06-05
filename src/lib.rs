#![forbid(unsafe_code)]

//! Prediction market for fleet intelligence with ternary confidence staking.

/// Ternary value representing a prediction or confidence direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ternary {
    Neg = -1,
    Zero = 0,
    Pos = 1,
}

impl Ternary {
    pub fn from_i8(v: i8) -> Option<Self> {
        match v {
            -1 => Some(Ternary::Neg),
            0 => Some(Ternary::Zero),
            1 => Some(Ternary::Pos),
            _ => None,
        }
    }

    pub fn to_i8(self) -> i8 {
        self as i8
    }
}

/// Unique identifier for agents and predictions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AgentId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PredictionId(pub u64);

/// The prediction service: manages markets and tracks predictions.
#[derive(Clone, Debug)]
pub struct Oracle {
    markets: Vec<PredictionMarket>,
    reputations: Vec<OracleReputation>,
    next_id: u64,
}

impl Oracle {
    pub fn new() -> Self {
        Oracle {
            markets: Vec::new(),
            reputations: Vec::new(),
            next_id: 0,
        }
    }

    /// Create a new prediction market for a given question.
    pub fn create_market(&mut self, question: &str) -> PredictionId {
        let id = PredictionId(self.next_id);
        self.next_id += 1;
        self.markets.push(PredictionMarket {
            id,
            question: question.to_string(),
            stakes: Vec::new(),
            resolved: false,
            outcome: None,
        });
        id
    }

    /// Place a stake on a prediction.
    pub fn stake(&mut self, market_id: PredictionId, agent: AgentId, prediction: Ternary, amount: u64) -> bool {
        if let Some(market) = self.markets.iter_mut().find(|m| m.id == market_id) {
            if market.resolved {
                return false;
            }
            market.stakes.push(OracleStake {
                agent,
                prediction,
                amount,
            });
            return true;
        }
        false
    }

    /// Resolve a market with a given outcome.
    pub fn resolve(&mut self, market_id: PredictionId, outcome: Ternary) -> bool {
        if let Some(market) = self.markets.iter_mut().find(|m| m.id == market_id) {
            if market.resolved {
                return false;
            }
            market.resolved = true;
            market.outcome = Some(outcome);
            return true;
        }
        false
    }

    pub fn markets(&self) -> &[PredictionMarket] {
        &self.markets
    }

    pub fn market(&self, id: PredictionId) -> Option<&PredictionMarket> {
        self.markets.iter().find(|m| m.id == id)
    }
}

/// A market where agents bet on outcomes.
#[derive(Clone, Debug)]
pub struct PredictionMarket {
    pub id: PredictionId,
    pub question: String,
    pub stakes: Vec<OracleStake>,
    pub resolved: bool,
    pub outcome: Option<Ternary>,
}

impl PredictionMarket {
    /// Calculate the total stake amount.
    pub fn total_stake(&self) -> u64 {
        self.stakes.iter().map(|s| s.amount).sum()
    }

    /// Get stakes for a specific prediction direction.
    pub fn stakes_for(&self, prediction: Ternary) -> Vec<&OracleStake> {
        self.stakes.iter().filter(|s| s.prediction == prediction).collect()
    }

    /// Determine the weighted consensus prediction.
    pub fn weighted_consensus(&self) -> Ternary {
        let mut scores: [i64; 3] = [0, 0, 0]; // neg, zero, pos
        for stake in &self.stakes {
            let idx = (stake.prediction.to_i8() + 1) as usize;
            scores[idx] += stake.amount as i64;
        }
        if scores[0] > scores[1] && scores[0] > scores[2] {
            Ternary::Neg
        } else if scores[2] > scores[1] && scores[2] > scores[0] {
            Ternary::Pos
        } else {
            Ternary::Zero
        }
    }
}

/// An agent's stake (confidence bet) on a prediction.
#[derive(Clone, Debug)]
pub struct OracleStake {
    pub agent: AgentId,
    pub prediction: Ternary,
    pub amount: u64,
}

impl OracleStake {
    pub fn new(agent: AgentId, prediction: Ternary, amount: u64) -> Self {
        OracleStake { agent, prediction, amount }
    }
}

/// Determines correct predictions by resolving markets.
#[derive(Clone, Debug)]
pub struct ResolutionMechanism;

impl ResolutionMechanism {
    /// Calculate payouts for a resolved market.
    pub fn resolve_market(market: &PredictionMarket) -> Vec<(AgentId, u64)> {
        if !market.resolved || market.outcome.is_none() {
            return Vec::new();
        }
        let outcome = market.outcome.unwrap();
        let winners: Vec<&OracleStake> = market.stakes.iter()
            .filter(|s| s.prediction == outcome)
            .collect();
        let losers_total: u64 = market.stakes.iter()
            .filter(|s| s.prediction != outcome)
            .map(|s| s.amount)
            .sum();

        if winners.is_empty() || losers_total == 0 {
            return winners.iter().map(|w| (w.agent, w.amount)).collect();
        }

        let winners_total: u64 = winners.iter().map(|w| w.amount).sum();
        winners.iter().map(|w| {
            let share = (w.amount as u128 * losers_total as u128 / winners_total as u128) as u64;
            (w.agent, w.amount + share)
        }).collect()
    }
}

/// Tracks predictor accuracy over time.
#[derive(Clone, Debug)]
pub struct OracleReputation {
    pub agent: AgentId,
    pub predictions: u64,
    pub correct: u64,
}

impl OracleReputation {
    pub fn new(agent: AgentId) -> Self {
        OracleReputation { agent, predictions: 0, correct: 0 }
    }

    /// Record a prediction result.
    pub fn record(&mut self, was_correct: bool) {
        self.predictions += 1;
        if was_correct {
            self.correct += 1;
        }
    }

    /// Get accuracy as a ratio (0.0 to 1.0).
    pub fn accuracy(&self) -> f64 {
        if self.predictions == 0 {
            0.0
        } else {
            self.correct as f64 / self.predictions as f64
        }
    }

    /// Get reputation weight (higher accuracy = higher weight).
    pub fn weight(&self) -> f64 {
        self.accuracy() * self.predictions as f64
    }
}

/// Aggregates predictions weighted by reputation.
#[derive(Clone, Debug)]
pub struct OracleConsensus;

impl OracleConsensus {
    /// Compute weighted consensus from a list of (prediction, weight) pairs.
    pub fn weighted_vote(votes: &[(Ternary, f64)]) -> Ternary {
        let mut score = 0.0;
        for (pred, weight) in votes {
            score += pred.to_i8() as f64 * weight;
        }
        if score < -0.001 {
            Ternary::Neg
        } else if score > 0.001 {
            Ternary::Pos
        } else {
            Ternary::Zero
        }
    }

    /// Aggregate predictions from stakes weighted by reputation.
    pub fn aggregate_with_reputation(
        stakes: &[OracleStake],
        reputations: &[OracleReputation],
    ) -> Ternary {
        let votes: Vec<(Ternary, f64)> = stakes.iter().map(|s| {
            let weight = reputations.iter()
                .find(|r| r.agent == s.agent)
                .map(|r| r.weight())
                .unwrap_or(1.0);
            (s.prediction, weight * s.amount as f64)
        }).collect();
        Self::weighted_vote(&votes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ternary_values() {
        assert_eq!(Ternary::from_i8(-1), Some(Ternary::Neg));
        assert_eq!(Ternary::from_i8(0), Some(Ternary::Zero));
        assert_eq!(Ternary::from_i8(1), Some(Ternary::Pos));
        assert_eq!(Ternary::from_i8(5), None);
    }

    #[test]
    fn test_oracle_create_market() {
        let mut oracle = Oracle::new();
        let id = oracle.create_market("Will room-3 deploy today?");
        assert!(oracle.market(id).is_some());
        assert_eq!(oracle.markets().len(), 1);
    }

    #[test]
    fn test_oracle_stake() {
        let mut oracle = Oracle::new();
        let id = oracle.create_market("Test?");
        assert!(oracle.stake(id, AgentId(1), Ternary::Pos, 100));
        let market = oracle.market(id).unwrap();
        assert_eq!(market.stakes.len(), 1);
    }

    #[test]
    fn test_oracle_stake_resolved_fails() {
        let mut oracle = Oracle::new();
        let id = oracle.create_market("Test?");
        oracle.resolve(id, Ternary::Pos);
        assert!(!oracle.stake(id, AgentId(1), Ternary::Pos, 100));
    }

    #[test]
    fn test_oracle_resolve() {
        let mut oracle = Oracle::new();
        let id = oracle.create_market("Test?");
        assert!(oracle.resolve(id, Ternary::Neg));
        let market = oracle.market(id).unwrap();
        assert!(market.resolved);
        assert_eq!(market.outcome, Some(Ternary::Neg));
    }

    #[test]
    fn test_market_total_stake() {
        let mut oracle = Oracle::new();
        let id = oracle.create_market("Test?");
        oracle.stake(id, AgentId(1), Ternary::Pos, 50);
        oracle.stake(id, AgentId(2), Ternary::Neg, 30);
        assert_eq!(oracle.market(id).unwrap().total_stake(), 80);
    }

    #[test]
    fn test_market_stakes_for() {
        let mut oracle = Oracle::new();
        let id = oracle.create_market("Test?");
        oracle.stake(id, AgentId(1), Ternary::Pos, 50);
        oracle.stake(id, AgentId(2), Ternary::Pos, 30);
        oracle.stake(id, AgentId(3), Ternary::Neg, 20);
        assert_eq!(oracle.market(id).unwrap().stakes_for(Ternary::Pos).len(), 2);
        assert_eq!(oracle.market(id).unwrap().stakes_for(Ternary::Neg).len(), 1);
    }

    #[test]
    fn test_market_weighted_consensus() {
        let mut oracle = Oracle::new();
        let id = oracle.create_market("Test?");
        oracle.stake(id, AgentId(1), Ternary::Pos, 100);
        oracle.stake(id, AgentId(2), Ternary::Neg, 30);
        assert_eq!(oracle.market(id).unwrap().weighted_consensus(), Ternary::Pos);
    }

    #[test]
    fn test_market_weighted_consensus_tie() {
        let mut oracle = Oracle::new();
        let id = oracle.create_market("Test?");
        oracle.stake(id, AgentId(1), Ternary::Pos, 50);
        oracle.stake(id, AgentId(2), Ternary::Neg, 50);
        assert_eq!(oracle.market(id).unwrap().weighted_consensus(), Ternary::Zero);
    }

    #[test]
    fn test_resolution_payout() {
        let mut oracle = Oracle::new();
        let id = oracle.create_market("Test?");
        oracle.stake(id, AgentId(1), Ternary::Pos, 100);
        oracle.stake(id, AgentId(2), Ternary::Neg, 100);
        oracle.resolve(id, Ternary::Pos);
        let payouts = ResolutionMechanism::resolve_market(oracle.market(id).unwrap());
        assert_eq!(payouts.len(), 1);
        assert_eq!(payouts[0].0, AgentId(1));
        assert_eq!(payouts[0].1, 200); // original + loser's share
    }

    #[test]
    fn test_resolution_unresolved() {
        let mut oracle = Oracle::new();
        let id = oracle.create_market("Test?");
        oracle.stake(id, AgentId(1), Ternary::Pos, 100);
        let payouts = ResolutionMechanism::resolve_market(oracle.market(id).unwrap());
        assert!(payouts.is_empty());
    }

    #[test]
    fn test_reputation_accuracy() {
        let mut rep = OracleReputation::new(AgentId(1));
        rep.record(true);
        rep.record(true);
        rep.record(false);
        assert_eq!(rep.predictions, 3);
        assert_eq!(rep.correct, 2);
        assert!((rep.accuracy() - 0.6667).abs() < 0.01);
    }

    #[test]
    fn test_reputation_weight() {
        let mut rep = OracleReputation::new(AgentId(1));
        rep.record(true);
        rep.record(true);
        assert!(rep.weight() > 0.0);
    }

    #[test]
    fn test_reputation_zero_predictions() {
        let rep = OracleReputation::new(AgentId(1));
        assert_eq!(rep.accuracy(), 0.0);
        assert_eq!(rep.weight(), 0.0);
    }

    #[test]
    fn test_consensus_weighted_vote() {
        let votes = vec![(Ternary::Pos, 3.0), (Ternary::Neg, 1.0)];
        assert_eq!(OracleConsensus::weighted_vote(&votes), Ternary::Pos);
    }

    #[test]
    fn test_consensus_weighted_vote_tie() {
        let votes = vec![(Ternary::Pos, 1.0), (Ternary::Neg, 1.0)];
        assert_eq!(OracleConsensus::weighted_vote(&votes), Ternary::Zero);
    }

    #[test]
    fn test_consensus_aggregate_with_reputation() {
        let stakes = vec![
            OracleStake::new(AgentId(1), Ternary::Pos, 100),
            OracleStake::new(AgentId(2), Ternary::Neg, 50),
        ];
        let mut rep1 = OracleReputation::new(AgentId(1));
        rep1.record(true); rep1.record(true); rep1.record(true);
        let mut rep2 = OracleReputation::new(AgentId(2));
        rep2.record(false); rep2.record(false);
        let reps = vec![rep1, rep2];
        let result = OracleConsensus::aggregate_with_reputation(&stakes, &reps);
        assert_eq!(result, Ternary::Pos);
    }

    #[test]
    fn test_oracle_stake_new() {
        let stake = OracleStake::new(AgentId(42), Ternary::Zero, 500);
        assert_eq!(stake.agent, AgentId(42));
        assert_eq!(stake.prediction, Ternary::Zero);
        assert_eq!(stake.amount, 500);
    }

    #[test]
    fn test_resolution_all_winners() {
        let mut oracle = Oracle::new();
        let id = oracle.create_market("Test?");
        oracle.stake(id, AgentId(1), Ternary::Pos, 100);
        oracle.stake(id, AgentId(2), Ternary::Pos, 50);
        oracle.resolve(id, Ternary::Pos);
        let payouts = ResolutionMechanism::resolve_market(oracle.market(id).unwrap());
        assert_eq!(payouts.len(), 2);
        // Both get their stake back, no losers to split
        assert_eq!(payouts[0].1, 100);
        assert_eq!(payouts[1].1, 50);
    }

    #[test]
    fn test_oracle_double_resolve_fails() {
        let mut oracle = Oracle::new();
        let id = oracle.create_market("Test?");
        assert!(oracle.resolve(id, Ternary::Pos));
        assert!(!oracle.resolve(id, Ternary::Neg));
    }
}
