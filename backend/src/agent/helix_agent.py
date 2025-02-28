# src/agent/protocol_agents/helix_agent.py
"""
Helix Protocol Agent

This module implements the agent for analyzing the Helix protocol
and making yield optimization recommendations.
"""

import logging
import numpy as np
from typing import Dict, List, Any, Optional, Tuple
from datetime import datetime

from src.agent.base_agent import BaseAgent
from src.adapters.helix_adapter import HelixAdapter


class HelixAgent(BaseAgent):
    """Agent for Helix protocol analysis and recommendations"""

    def __init__(self, adapter: Optional[HelixAdapter] = None):
        """
        Initialize Helix agent

        Args:
            adapter: Helix adapter instance (optional)
        """
        super().__init__("helix")
        self.adapter = adapter or HelixAdapter()

    async def analyze_opportunities(self) -> Dict[str, Any]:
        """
        Analyze yield opportunities in Helix protocol

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

                # Add metrics to pool data
                opportunity = {
                    **pool,
                    "apy_volatility": round(apy_volatility, 4),
                    "risk_adjusted_return": round(risk_adjusted_return, 2),
                    "analysis_timestamp": datetime.now().isoformat(),
                }

                opportunities.append(opportunity)

            # Sort by risk-adjusted return
            opportunities.sort(
                key=lambda x: x.get("risk_adjusted_return", 0), reverse=True
            )

            # Get protocol info
            protocol_info = await self.adapter.get_protocol_info()

            return {
                "protocol": {
                    "id": self.protocol_id,
                    "name": protocol_info.get("name", "Helix"),
                    "total_tvl": sum(pool.get("tvl", 0) for pool in pools),
                    "pool_count": len(pools),
                },
                "opportunities": opportunities,
                "top_opportunity": opportunities[0] if opportunities else None,
                "analysis_timestamp": datetime.now().isoformat(),
            }

        except Exception as e:
            self.logger.error(f"Error analyzing opportunities: {e}")
            return {
                "protocol": {"id": self.protocol_id, "name": "Helix", "error": str(e)},
                "opportunities": [],
                "analysis_timestamp": datetime.now().isoformat(),
            }

    async def get_risk_assessment(self) -> Dict[str, Any]:
        """
        Get risk assessment for Helix protocol

        Returns:
            Risk assessment metrics
        """
        try:
            # Get protocol risk from adapter
            protocol_risk = await self.adapter.estimate_protocol_risk()

            # Get pools to assess pool-specific risks
            pools = await self.adapter.get_pools()

            # Calculate average risk metrics
            avg_pool_risk = (
                sum(pool.get("risk_score", 5.0) for pool in pools) / len(pools)
                if pools
                else 5.0
            )

            # Get TVL for size-based risk
            total_tvl = sum(pool.get("tvl", 0) for pool in pools)
            tvl_risk = max(1.0, 10.0 - min(9.0, total_tvl / 1_000_000 * 0.5))

            # Calculate compositional risk based on asset types
            compositional_risk = self._assess_compositional_risk(pools)

            # Combine risk factors into overall risk
            weights = {"protocol": 0.3, "pools": 0.3, "tvl": 0.2, "compositional": 0.2}

            overall_risk = (
                protocol_risk * weights["protocol"]
                + avg_pool_risk * weights["pools"]
                + tvl_risk * weights["tvl"]
                + compositional_risk * weights["compositional"]
            )

            return {
                "protocol_id": self.protocol_id,
                "overall_risk": round(overall_risk, 1),
                "protocol_risk": protocol_risk,
                "average_pool_risk": round(avg_pool_risk, 1),
                "tvl_risk": round(tvl_risk, 1),
                "compositional_risk": round(compositional_risk, 1),
                "assessment_timestamp": datetime.now().isoformat(),
            }

        except Exception as e:
            self.logger.error(f"Error getting risk assessment: {e}")
            return {
                "protocol_id": self.protocol_id,
                "overall_risk": 5.0,  # Default moderate risk
                "error": str(e),
                "assessment_timestamp": datetime.now().isoformat(),
            }

    def _assess_compositional_risk(self, pools: List[Dict[str, Any]]) -> float:
        """
        Assess risk based on asset composition of pools

        Args:
            pools: List of pool data

        Returns:
            Compositional risk score
        """
        # Simplified implementation - in production would analyze token correlations

        # Count pools by type
        stable_pools = 0
        volatile_pools = 0

        for pool in pools:
            pair = pool.get("pair", [])
            # Check if both tokens are stablecoins
            if len(pair) == 2 and all(
                token in ["USDC", "USDT", "DAI"] for token in pair
            ):
                stable_pools += 1
            else:
                volatile_pools += 1

        total_pools = stable_pools + volatile_pools
        if total_pools == 0:
            return 5.0  # Default moderate risk

        # More stable pools = lower risk
        stable_ratio = stable_pools / total_pools

        # Scale to risk score (higher stable ratio = lower risk)
        compositional_risk = 8.0 - (stable_ratio * 5.0)

        return compositional_risk

    async def get_recommended_pools(
        self, risk_profile: str = "moderate"
    ) -> List[Dict[str, Any]]:
        """
        Get recommended Helix pools based on risk profile

        Args:
            risk_profile: Risk profile (conservative, moderate, aggressive)

        Returns:
            List of recommended pools with rationale
        """
        try:
            # Get opportunities analysis
            analysis = await self.analyze_opportunities()
            opportunities = analysis.get("opportunities", [])

            if not opportunities:
                return []

            # Define risk thresholds based on profile
            risk_thresholds = {"conservative": 4.0, "moderate": 6.0, "aggressive": 8.0}

            risk_threshold = risk_thresholds.get(risk_profile, 6.0)

            # Filter pools by risk threshold
            eligible_pools = [
                pool
                for pool in opportunities
                if pool.get("risk_score", 5.0) <= risk_threshold
            ]

            if not eligible_pools:
                # If no pools meet the criteria, return top opportunities with warning
                recommendations = opportunities[:3]
                for rec in recommendations:
                    rec["recommendation_note"] = (
                        "Included despite exceeding risk threshold due to limited options"
                    )
            else:
                # Sort by appropriate metric based on risk profile
                if risk_profile == "conservative":
                    # Sort by risk score (ascending) then APY
                    eligible_pools.sort(
                        key=lambda x: (x.get("risk_score", 10), -x.get("apy", 0))
                    )
                elif risk_profile == "aggressive":
                    # Sort by APY (descending)
                    eligible_pools.sort(key=lambda x: x.get("apy", 0), reverse=True)
                else:
                    # Sort by risk-adjusted return
                    eligible_pools.sort(
                        key=lambda x: x.get("risk_adjusted_return", 0), reverse=True
                    )

                # Take top recommendations
                recommendations = eligible_pools[:3]

                # Add recommendation notes
                for rec in recommendations:
                    if risk_profile == "conservative":
                        rec["recommendation_note"] = (
                            f"Low risk option with {rec.get('apy', 0)}% APY"
                        )
                    elif risk_profile == "aggressive":
                        rec["recommendation_note"] = (
                            f"High yield option with strong returns"
                        )
                    else:
                        rec["recommendation_note"] = f"Balanced risk-reward profile"

            return recommendations

        except Exception as e:
            self.logger.error(f"Error getting recommended pools: {e}")
            return []

    async def estimate_correlation(self, other_protocol_id: str) -> float:
        """
        Estimate correlation between Helix and another protocol

        Args:
            other_protocol_id: ID of the other protocol

        Returns:
            Correlation coefficient (-1 to 1)
        """
        # This is a simplified implementation - in production would analyze price and APY correlations

        # Placeholder correlation estimates based on protocol characteristics
        correlations = {
            "hydro": 0.6,  # Moderate correlation with Hydro
            "neptune": 0.4,  # Lower correlation with Neptune
        }

        return correlations.get(other_protocol_id, 0.5)  # Default moderate correlation

    async def get_historical_performance(self, days: int = 30) -> Dict[str, Any]:
        """
        Get historical performance metrics for Helix

        Args:
            days: Number of days of history

        Returns:
            Historical performance metrics
        """
        try:
            # Get pools
            pools = await self.adapter.get_pools()

            # Get historical data for each pool
            historical_data = {}
            for pool in pools:
                pool_id = pool["id"]
                try:
                    history = await self.adapter.get_historical_data(pool_id, days)
                    historical_data[pool_id] = history
                except Exception as e:
                    self.logger.warning(
                        f"Error getting history for pool {pool_id}: {e}"
                    )

            # Calculate protocol-level performance metrics
            daily_metrics = []
            for i in range(min(days, len(next(iter(historical_data.values()), [])))):
                day_data = {"date": "", "total_tvl": 0, "avg_apy": 0, "volume": 0}

                # Collect metrics for this day across all pools
                tvl_values = []
                apy_values = []
                volume_values = []

                for pool_id, history in historical_data.items():
                    if i < len(history):
                        day = history[i]
                        # Set date from first pool (they should all be the same)
                        if not day_data["date"]:
                            day_data["date"] = day.get("date", "")

                        tvl_values.append(day.get("tvl", 0))
                        apy_values.append(day.get("apy", 0))
                        volume_values.append(day.get("volume_24h", 0))

                # Calculate aggregates
                day_data["total_tvl"] = sum(tvl_values)
                day_data["avg_apy"] = (
                    sum(apy_values) / len(apy_values) if apy_values else 0
                )
                day_data["volume"] = sum(volume_values)

                daily_metrics.append(day_data)

            return {
                "protocol_id": self.protocol_id,
                "days": days,
                "pools": historical_data,
                "protocol_metrics": daily_metrics,
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
