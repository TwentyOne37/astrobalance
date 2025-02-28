# src/agent/protocol_agents/neptune_agent.py
"""
Neptune Protocol Agent

This module implements the agent for analyzing the Neptune protocol
and making yield optimization recommendations.
"""

import logging
import numpy as np
from typing import Dict, List, Any, Optional, Tuple
from datetime import datetime

from src.agent.base_agent import BaseAgent
from src.adapters.neptune_adapter import NeptuneAdapter


class NeptuneAgent(BaseAgent):
    """Agent for Neptune protocol analysis and recommendations"""

    def __init__(self, adapter: Optional[NeptuneAdapter] = None):
        """
        Initialize Neptune agent

        Args:
            adapter: Neptune adapter instance (optional)
        """
        super().__init__("neptune")
        self.adapter = adapter or NeptuneAdapter()

    async def analyze_opportunities(self) -> Dict[str, Any]:
        """
        Analyze yield opportunities in Neptune protocol

        Neptune is a yield aggregator, so we focus on strategy aspects

        Returns:
            Dictionary of opportunities with analysis
        """
        try:
            # Get pools from adapter
            pools = await self.adapter.get_pools()

            # Get some historical data for volatility analysis
            history_data = {}
            for pool in pools:
                pool_id = pool["id"]
                try:
                    history = await self.adapter.get_historical_data(pool_id, days=14)
                    history_data[pool_id] = history
                except Exception as e:
                    self.logger.warning(
                        f"Error getting history for pool {pool_id}: {e}"
                    )

            # For Neptune, get strategy details for each pool
            strategy_data = {}
            for pool in pools:
                pool_id = pool["id"]
                try:
                    strategy = await self.adapter.get_strategy_details(pool_id)
                    strategy_data[pool_id] = strategy
                except Exception as e:
                    self.logger.warning(
                        f"Error getting strategy for pool {pool_id}: {e}"
                    )

            # Calculate additional metrics for each pool
            opportunities = []
            for pool in pools:
                pool_id = pool["id"]

                # Get historical APY values if available
                apy_history = []
                if pool_id in history_data:
                    apy_history = [day.get("apy", 0) for day in history_data[pool_id]]

                # Calculate APY volatility
                apy_volatility = (
                    np.std(apy_history) / np.mean(apy_history)
                    if apy_history and np.mean(apy_history) > 0
                    else 0
                )

                # Calculate risk-adjusted return (APY / risk_score)
                risk_score = pool.get("risk_score", 5.0)
                risk_adjusted_return = (
                    pool.get("apy", 0) / risk_score if risk_score > 0 else 0
                )

                # For Neptune, get strategy details
                strategy = strategy_data.get(pool_id, {})
                strategy_type = strategy.get("strategy_type", "Unknown")

                # Neptune-specific metric: Strategy complexity penalty
                # More complex strategies have higher risk
                strategy_complexity = {
                    "LP Compounding": 1.0,  # Simple strategy
                    "Yield Optimizer": 1.5,  # Medium complexity
                    "Multi-protocol": 2.0,  # High complexity
                }.get(strategy_type, 1.5)

                # Adjust risk score based on strategy complexity
                adjusted_risk = risk_score * strategy_complexity

                # Add metrics to pool data
                opportunity = {
                    **pool,
                    "apy_volatility": round(apy_volatility, 4),
                    "risk_adjusted_return": round(risk_adjusted_return, 2),
                    "strategy_type": strategy_type,
                    "strategy_complexity": strategy_complexity,
                    "adjusted_risk": round(adjusted_risk, 2),
                    "analysis_timestamp": datetime.now().isoformat(),
                }

                # Add any strategy-specific metrics
                if strategy:
                    opportunity["performance_fee"] = strategy.get(
                        "performance_fee", "10%"
                    )
                    opportunity["harvesting_frequency"] = strategy.get(
                        "harvesting_frequency", "Daily"
                    )
                    opportunity["underlying_protocol"] = strategy.get(
                        "underlying_protocol", ""
                    )

                opportunities.append(opportunity)

            # Sort by risk-adjusted return, penalizing for strategy complexity
            opportunities.sort(
                key=lambda x: x.get("risk_adjusted_return", 0)
                / x.get("strategy_complexity", 1.0),
                reverse=True,
            )

            # Get protocol info
            protocol_info = await self.adapter.get_protocol_info()

            # Group by strategy type (a Neptune-specific feature)
            strategy_groups = {}
            for opp in opportunities:
                strategy_type = opp.get("strategy_type", "Unknown")
                if strategy_type not in strategy_groups:
                    strategy_groups[strategy_type] = []
                strategy_groups[strategy_type].append(opp)

            return {
                "protocol": {
                    "id": self.protocol_id,
                    "name": protocol_info.get("name", "Neptune Finance"),
                    "total_tvl": sum(pool.get("tvl", 0) for pool in pools),
                    "pool_count": len(pools),
                },
                "opportunities": opportunities,
                "strategy_groups": strategy_groups,
                "top_opportunity": opportunities[0] if opportunities else None,
                "analysis_timestamp": datetime.now().isoformat(),
            }

        except Exception as e:
            self.logger.error(f"Error analyzing opportunities: {e}")
            return {
                "protocol": {
                    "id": self.protocol_id,
                    "name": "Neptune Finance",
                    "error": str(e),
                },
                "opportunities": [],
                "analysis_timestamp": datetime.now().isoformat(),
            }

    async def get_risk_assessment(self) -> Dict[str, Any]:
        """
        Get risk assessment for Neptune protocol

        Neptune has additional risk factors related to yield aggregation strategies

        Returns:
            Risk assessment metrics
        """
        try:
            # Get protocol risk from adapter
            protocol_risk = await self.adapter.estimate_protocol_risk()

            # Get pools to assess pool-specific risks
            pools = await self.adapter.get_pools()

            # For Neptune, get strategy details to assess strategy risk
            strategy_risks = []
            for pool in pools:
                pool_id = pool["id"]
                try:
                    strategy = await self.adapter.get_strategy_details(pool_id)

                    # Assess strategy risk based on complexity
                    strategy_type = strategy.get("strategy_type", "")

                    strategy_risk_factor = {
                        "LP Compounding": 1.0,  # Lowest risk
                        "Yield Optimizer": 1.3,  # Medium risk
                        "Multi-protocol": 1.5,  # Highest risk
                    }.get(strategy_type, 1.3)

                    strategy_risks.append(strategy_risk_factor)

                except Exception as e:
                    self.logger.warning(
                        f"Error getting strategy for pool {pool_id}: {e}"
                    )
                    strategy_risks.append(1.3)  # Default medium risk

            # Calculate average strategy risk
            avg_strategy_risk = (
                sum(strategy_risks) / len(strategy_risks) if strategy_risks else 1.3
            )

            # Calculate average pool risk
            avg_pool_risk = (
                sum(pool.get("risk_score", 5.0) for pool in pools) / len(pools)
                if pools
                else 5.0
            )

            # Adjust for Neptune's strategy approach
            adjusted_pool_risk = avg_pool_risk * avg_strategy_risk

            # Get TVL for size-based risk
            total_tvl = sum(pool.get("tvl", 0) for pool in pools)
            tvl_risk = max(1.0, 10.0 - min(9.0, total_tvl / 1_000_000 * 0.5))

            # Neptune-specific: smart contract risk (newer protocol)
            smart_contract_risk = (
                6.0  # Higher than average due to newer, more complex protocols
            )

            # Neptune-specific: dependency risk (relies on underlying protocols)
            dependency_risk = 7.0  # Higher risk due to dependence on multiple protocols

            # Combine risk factors into overall risk
            weights = {
                "protocol": 0.2,
                "pools": 0.2,
                "tvl": 0.1,
                "strategy": 0.2,
                "smart_contract": 0.15,
                "dependency": 0.15,
            }

            overall_risk = (
                protocol_risk * weights["protocol"]
                + adjusted_pool_risk * weights["pools"]
                + tvl_risk * weights["tvl"]
                + (avg_strategy_risk * 4.0)
                * weights["strategy"]  # Scale to similar range
                + smart_contract_risk * weights["smart_contract"]
                + dependency_risk * weights["dependency"]
            )

            return {
                "protocol_id": self.protocol_id,
                "overall_risk": round(overall_risk, 1),
                "protocol_risk": protocol_risk,
                "average_pool_risk": round(avg_pool_risk, 1),
                "adjusted_pool_risk": round(adjusted_pool_risk, 1),
                "average_strategy_risk": round(avg_strategy_risk, 2),
                "tvl_risk": round(tvl_risk, 1),
                "smart_contract_risk": smart_contract_risk,
                "dependency_risk": dependency_risk,
                "assessment_timestamp": datetime.now().isoformat(),
            }

        except Exception as e:
            self.logger.error(f"Error getting risk assessment: {e}")
            return {
                "protocol_id": self.protocol_id,
                "overall_risk": 6.0,  # Default slightly above moderate risk for Neptune
                "error": str(e),
                "assessment_timestamp": datetime.now().isoformat(),
            }

    async def get_recommended_pools(
        self, risk_profile: str = "moderate"
    ) -> List[Dict[str, Any]]:
        """
        Get recommended Neptune pools based on risk profile

        For Neptune, we consider strategy complexity in recommendations

        Args:
            risk_profile: Risk profile (conservative, moderate, aggressive)

        Returns:
            List of recommended pools with rationale
        """
        try:
            # Get opportunities analysis
            analysis = await self.analyze_opportunities()
            opportunities = analysis.get("opportunities", [])
            strategy_groups = analysis.get("strategy_groups", {})

            if not opportunities:
                return []

            # Define risk thresholds based on profile
            # Neptune has higher base risk due to strategy complexity
            risk_thresholds = {"conservative": 5.0, "moderate": 6.5, "aggressive": 8.0}

            risk_threshold = risk_thresholds.get(risk_profile, 6.5)

            # For Neptune, filter based on adjusted risk that considers strategy complexity
            eligible_pools = [
                pool
                for pool in opportunities
                if pool.get("adjusted_risk", 5.0) <= risk_threshold
            ]

            # If we don't have enough options, try regular risk score
            if len(eligible_pools) < 2:
                eligible_pools = [
                    pool
                    for pool in opportunities
                    if pool.get("risk_score", 5.0) <= risk_threshold
                ]

            # For Neptune, different sorting strategies based on risk profile
            if risk_profile == "conservative":
                # For conservative, prioritize simpler strategies with lower volatility
                eligible_pools.sort(
                    key=lambda x: (
                        x.get("strategy_complexity", 2.0),  # Simpler first
                        x.get("apy_volatility", 1.0),  # Lower volatility second
                        -x.get("apy", 0),  # Higher APY third
                    )
                )

            elif risk_profile == "aggressive":
                # For aggressive, prioritize pure APY
                eligible_pools.sort(key=lambda x: x.get("apy", 0), reverse=True)

            else:  # moderate
                # For moderate, balance risk-adjusted return and strategy complexity
                eligible_pools.sort(
                    key=lambda x: (
                        -x.get(
                            "risk_adjusted_return", 0
                        ),  # Higher risk-adjusted return first
                        x.get("strategy_complexity", 2.0),  # Lower complexity second
                    )
                )

            # If no pools meet criteria, return top opportunities with warning
            if not eligible_pools:
                recommendations = opportunities[:3]
                for rec in recommendations:
                    rec["recommendation_note"] = (
                        "Included despite exceeding risk threshold due to limited options"
                    )
            else:
                # Take top recommendations
                recommendations = eligible_pools[:3]

                # Add recommendation notes
                for rec in recommendations:
                    strategy_type = rec.get("strategy_type", "Unknown")

                    if risk_profile == "conservative":
                        if strategy_type == "LP Compounding":
                            rec["recommendation_note"] = (
                                f"Simple strategy with {rec.get('apy', 0)}% APY and lower complexity"
                            )
                        else:
                            rec["recommendation_note"] = (
                                f"Relatively stable strategy despite some complexity"
                            )
                    elif risk_profile == "aggressive":
                        if strategy_type == "Multi-protocol":
                            rec["recommendation_note"] = (
                                f"Complex strategy with high yield potential"
                            )
                        else:
                            rec["recommendation_note"] = (
                                f"Strong yield option with good performance history"
                            )
                    else:
                        rec["recommendation_note"] = (
                            f"Balanced risk-reward profile with {strategy_type} strategy"
                        )

            return recommendations

        except Exception as e:
            self.logger.error(f"Error getting recommended pools: {e}")
            return []

    async def estimate_correlation(self, other_protocol_id: str) -> float:
        """
        Estimate correlation between Neptune and another protocol

        Args:
            other_protocol_id: ID of the other protocol

        Returns:
            Correlation coefficient (-1 to 1)
        """
        # Neptune tends to have medium correlations due to its aggregator nature

        # Placeholder correlation estimates based on protocol characteristics
        correlations = {
            "helix": 0.5,  # Medium correlation with Helix (often builds on it)
            "hydro": 0.4,  # Lower correlation with Hydro (different focus)
        }

        return correlations.get(other_protocol_id, 0.5)  # Default moderate correlation

    async def get_historical_performance(self, days: int = 30) -> Dict[str, Any]:
        """
        Get historical performance metrics for Neptune

        For Neptune, we include strategy-specific metrics

        Args:
            days: Number of days of history

        Returns:
            Historical performance metrics with strategy data
        """
        try:
            # Get pools
            pools = await self.adapter.get_pools()

            # Get historical data for each pool
            historical_data = {}
            for pool in pools:
                pool_id = pool["id"]
                try:
                    # For Neptune, use the specialized performance history method
                    history = await self.adapter.get_performance_history(pool_id, days)
                    historical_data[pool_id] = history.get("metrics", [])
                except Exception as e:
                    self.logger.warning(
                        f"Error getting history for pool {pool_id}: {e}"
                    )
                    # Fall back to regular historical data
                    try:
                        regular_history = await self.adapter.get_historical_data(
                            pool_id, days
                        )
                        historical_data[pool_id] = regular_history
                    except Exception as e2:
                        self.logger.error(
                            f"Error getting fallback history for pool {pool_id}: {e2}"
                        )

            # Get strategy data for each pool
            strategies = {}
            for pool in pools:
                pool_id = pool["id"]
                try:
                    strategy = await self.adapter.get_strategy_details(pool_id)
                    strategies[pool_id] = strategy
                except Exception as e:
                    self.logger.warning(
                        f"Error getting strategy for pool {pool_id}: {e}"
                    )

            # Calculate protocol-level performance metrics
            daily_metrics = []
            for i in range(min(days, len(next(iter(historical_data.values()), [])))):
                day_data = {
                    "date": "",
                    "total_tvl": 0,
                    "avg_apy": 0,
                    "harvests": 0,
                    "fees_collected": 0,
                }

                # Collect metrics for this day across all pools
                tvl_values = []
                apy_values = []
                harvest_values = []
                fees_values = []

                for pool_id, history in historical_data.items():
                    if i < len(history):
                        day = history[i]
                        # Set date from first pool (they should all be the same)
                        if not day_data["date"]:
                            day_data["date"] = day.get("date", "")

                        tvl_values.append(day.get("tvl", 0))
                        apy_values.append(day.get("apy", 0))

                        # Neptune-specific metrics
                        harvest_values.append(day.get("harvests", 0))
                        fees_values.append(day.get("fees_collected", 0))

                # Calculate aggregates
                day_data["total_tvl"] = sum(tvl_values)
                day_data["avg_apy"] = (
                    sum(apy_values) / len(apy_values) if apy_values else 0
                )
                day_data["harvests"] = sum(harvest_values)
                day_data["fees_collected"] = sum(fees_values)

                daily_metrics.append(day_data)

            # Group historical data by strategy type
            strategy_performance = {}
            for pool_id, strategy in strategies.items():
                strategy_type = strategy.get("strategy_type", "Unknown")

                if strategy_type not in strategy_performance:
                    strategy_performance[strategy_type] = {
                        "pools": [],
                        "avg_apy": 0,
                        "total_tvl": 0,
                        "performance": [],  # Will hold daily performance
                    }

                # Add pool to this strategy type
                pool = next((p for p in pools if p["id"] == pool_id), {})
                strategy_performance[strategy_type]["pools"].append(pool)

                # Add TVL and APY to strategy averages
                strategy_performance[strategy_type]["total_tvl"] += pool.get("tvl", 0)

            # Calculate average APYs for each strategy type
            for strategy_type, data in strategy_performance.items():
                pool_count = len(data["pools"])
                if pool_count > 0:
                    total_apy = sum(p.get("apy", 0) for p in data["pools"])
                    data["avg_apy"] = total_apy / pool_count

            return {
                "protocol_id": self.protocol_id,
                "days": days,
                "pools": historical_data,
                "protocol_metrics": daily_metrics,
                "strategies": strategies,
                "strategy_performance": strategy_performance,
                "timestamp": datetime.now().isoformat(),
            }

        except Exception as e:
            self.logger.error(f"Error getting historical performance: {e}")
            return {
                "protocol_id": self.protocol_id,
                "days": days,
                "error": str(e),
                "timestamp": datetime.now().isoformat(),
            }

    async def analyze_strategy_efficiency(self) -> Dict[str, Any]:
        """
        Analyze the efficiency of Neptune's yield strategies

        This is a Neptune-specific analysis that evaluates how efficiently
        the strategies harvest and compound yields

        Returns:
            Strategy efficiency metrics
        """
        try:
            # Get all pools
            pools = await self.adapter.get_pools()

            # Get strategies for all pools
            strategy_efficiencies = []
            for pool in pools:
                pool_id = pool["id"]
                try:
                    # Get strategy details
                    strategy = await self.adapter.get_strategy_details(pool_id)

                    # Get performance history for efficiency analysis
                    performance = await self.adapter.get_performance_history(
                        pool_id, days=7
                    )

                    # Calculate efficiency metrics
                    harvesting_frequency = strategy.get("harvesting_frequency", "")
                    avg_harvests_per_day = 0
                    avg_efficiency = 0

                    # Extract metrics from performance data
                    if "metrics" in performance:
                        metrics = performance.get("metrics", [])
                        if metrics:
                            # Calculate average harvests per day
                            harvest_counts = [m.get("harvests", 0) for m in metrics]
                            avg_harvests_per_day = sum(harvest_counts) / len(
                                harvest_counts
                            )

                            # Calculate average efficiency
                            efficiencies = [
                                m.get("strategy_efficiency", 0) for m in metrics
                            ]
                            avg_efficiency = (
                                sum(efficiencies) / len(efficiencies)
                                if efficiencies
                                else 0
                            )

                    # Calculate overall efficiency score
                    # Factors: Harvest frequency, efficiency score, fee impact

                    # Convert harvesting frequency to numeric score
                    harvest_score = {
                        "1 hour": 9.0,
                        "4 hours": 8.0,
                        "6 hours": 7.0,
                        "12 hours": 6.0,
                        "Daily": 5.0,
                    }.get(harvesting_frequency, 5.0)

                    # Performance fee impact (higher fee = lower score)
                    fee_str = strategy.get("performance_fee", "10%").replace("%", "")
                    try:
                        fee_pct = float(fee_str)
                    except:
                        fee_pct = 10.0

                    fee_impact = max(1.0, 10.0 - fee_pct / 2)  # 10% fee = 5.0 score

                    # Composite efficiency score
                    efficiency_score = (
                        (avg_efficiency * 10) * 0.5  # Strategy efficiency (50%)
                        + harvest_score * 0.3  # Harvest frequency (30%)
                        + fee_impact * 0.2  # Fee impact (20%)
                    )

                    strategy_efficiencies.append(
                        {
                            "pool_id": pool_id,
                            "name": pool.get("name", ""),
                            "strategy_type": strategy.get("strategy_type", ""),
                            "harvesting_frequency": harvesting_frequency,
                            "avg_harvests_per_day": round(avg_harvests_per_day, 2),
                            "avg_efficiency": round(avg_efficiency, 4),
                            "performance_fee": strategy.get("performance_fee", "10%"),
                            "apy": pool.get("apy", 0),
                            "underlying_protocol": strategy.get(
                                "underlying_protocol", ""
                            ),
                            "efficiency_score": round(efficiency_score, 2),
                        }
                    )

                except Exception as e:
                    self.logger.warning(
                        f"Error analyzing strategy for pool {pool_id}: {e}"
                    )

            # Sort by efficiency score
            strategy_efficiencies.sort(
                key=lambda x: x.get("efficiency_score", 0), reverse=True
            )

            # Calculate protocol-level metrics
            avg_efficiency = (
                sum(s.get("efficiency_score", 0) for s in strategy_efficiencies)
                / len(strategy_efficiencies)
                if strategy_efficiencies
                else 0
            )

            # Group by strategy type
            strategy_type_metrics = {}
            for strategy in strategy_efficiencies:
                strategy_type = strategy.get("strategy_type", "Unknown")

                if strategy_type not in strategy_type_metrics:
                    strategy_type_metrics[strategy_type] = {
                        "count": 0,
                        "total_efficiency": 0,
                        "strategies": [],
                    }

                strategy_type_metrics[strategy_type]["count"] += 1
                strategy_type_metrics[strategy_type]["total_efficiency"] += (
                    strategy.get("efficiency_score", 0)
                )
                strategy_type_metrics[strategy_type]["strategies"].append(strategy)

            # Calculate averages for each strategy type
            for strategy_type, metrics in strategy_type_metrics.items():
                if metrics["count"] > 0:
                    metrics["avg_efficiency"] = round(
                        metrics["total_efficiency"] / metrics["count"], 2
                    )

            return {
                "protocol_id": self.protocol_id,
                "strategy_count": len(strategy_efficiencies),
                "average_efficiency_score": round(avg_efficiency, 2),
                "top_strategy": strategy_efficiencies[0]
                if strategy_efficiencies
                else None,
                "strategy_efficiencies": strategy_efficiencies,
                "strategy_type_metrics": strategy_type_metrics,
                "timestamp": datetime.now().isoformat(),
            }

        except Exception as e:
            self.logger.error(f"Error analyzing strategy efficiency: {e}")
            return {
                "protocol_id": self.protocol_id,
                "error": str(e),
                "timestamp": datetime.now().isoformat(),
            }
