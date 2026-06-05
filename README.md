# ternary-oracle: Prediction market for fleet intelligence with ternary confidence staking

## Why This Exists

A fleet of agents generates a constant stream of predictions: "Will room-3 complete deployment by 17:00?", "Is the east cluster going to experience load spikes?", "Will agent-7's task succeed?" Without a structured way to collect and evaluate these predictions, institutional knowledge stays trapped in individual agents and is lost when they restart.

A prediction market gives agents skin in the game — they stake resources on their confidence, and their accuracy is tracked over time. This creates a reputation-weighted consensus that outperforms simple majority voting, because agents with a history of being right get more influence.

## Core Concepts

- **Ternary prediction**: Each stake is `Neg` (against), `Zero` (neutral), or `Pos` (for) — three levels of confidence direction.
- **Oracle**: The central prediction service. Creates markets, accepts stakes, and resolves outcomes.
- **PredictionMarket**: A single question with stakes from multiple agents. Resolves to one ternary outcome.
- **OracleStake**: An agent's bet: a ternary prediction paired with an amount of resources at risk.
- **ResolutionMechanism**: When a market resolves, it calculates payouts. Winners split the losers' stakes proportionally.
- **OracleReputation**: Per-agent accuracy tracking. Records how many predictions were correct out of total attempts.
- **OracleConsensus**: Aggregates predictions weighted by reputation and stake size, producing a single ternary result.

## Quick Start

```toml
[dependencies]
ternary-oracle = "0.1"
```

```rust
use ternary_oracle::*;

let mut oracle = Oracle::new();
let market = oracle.create_market("Will deployment succeed?");

oracle.stake(market, AgentId(1), Ternary::Pos, 100);
oracle.stake(market, AgentId(2), Ternary::Neg, 50);

// Later, resolve with the actual outcome
oracle.resolve(market, Ternary::Pos);

// Calculate payouts
let payouts = ResolutionMechanism::resolve_market(oracle.market(market).unwrap());
for (agent, amount) in payouts {
    println!("Agent {:?} receives {}", agent, amount);
}
```

## API Overview

| Type | Description |
|------|-------------|
| `Oracle` | Central prediction service managing markets and resolutions |
| `PredictionMarket` | A single prediction question with stakes and resolution state |
| `OracleStake` | An agent's bet: prediction direction + staked amount |
| `ResolutionMechanism` | Calculates payouts from resolved markets (winners split losers) |
| `OracleReputation` | Tracks per-agent prediction accuracy over time |
| `OracleConsensus` | Produces reputation-weighted aggregate predictions |
| `AgentId` / `PredictionId` | Unique identifiers for agents and markets |

## How It Works

When a market is created, it starts open and accepting stakes. Each stake records an agent's prediction direction and the amount they're wagering. The market's `weighted_consensus` method computes the direction with the highest total stake.

Resolution locks the market and determines payouts: agents who predicted the correct outcome split the total of incorrect stakes, proportional to their own wager. If all agents predicted correctly (or no one did), each simply gets their stake back.

Reputation tracking is separate from markets: an `OracleReputation` records raw hit/miss counts and exposes an accuracy ratio and a weight (accuracy × number of predictions). The `OracleConsensus` uses these weights to compute a reputation-weighted vote, so agents with better track records have more influence.

## Known Limitations

- No on-chain verification or cryptographic commitments — resolution is trusted.
- No time-based market expiry; markets stay open until explicitly resolved.
- Payouts use integer arithmetic and may have rounding errors when splitting loser stakes among many winners.
- Reputation is global, not per-topic — an agent accurate at predicting deployments gets equal weight when predicting network issues.
- No mechanism to slash or penalize agents who never resolve their predictions.

## Use Cases

- **Deployment predictions**: Agents stake on whether a room's deployment will succeed, building a collective confidence score.
- **Capacity forecasting**: Predict whether cluster X will hit 90% utilization in the next hour.
- **Task outcome betting**: Before assigning a complex task, let agents predict its success to surface hidden risk factors.
- **Reputation-based routing**: Use `OracleReputation` weights to prioritize listening to agents with proven accuracy.

## Ecosystem Context

Part of the SuperInstance ternary crate family. Complements `ternary-quorum` (formal voting) and `ternary-bayesian` (probabilistic reasoning). The oracle provides a market-based approach to collective intelligence, while quorum handles structured decision-making and bayesian handles evidence-based inference.

## License

MIT
