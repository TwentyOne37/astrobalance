"""
Hydro Protocol Agent

This module implements the agent for analyzing the Hydro protocol
and making yield optimization recommendations.
"""

import logging
import numpy as np
from typing import Dict, List, Any, Optional, Tuple
from datetime import datetime

from src.agent.base_agent import BaseAgent
from src.adapters.hydro_adapter import HydroAdapter


class HydroAgent(BaseAgent):
    """Agent for Hydro protocol analysis and recommendations"""

    def __init__(self, adapter: Optional[HydroAdapter] = None):
        """
        Initialize Hydro agent

        Args:
            adapter: Hydro adapter instance (optional)
        """
        super().__init__("hydro")
        self.adapter = adapter or HydroAdapter()

    async def analyze_opportunities(self) -> Dict[str, Any]:
        """
        Analyze yield opportunities in Hydro protocol

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

                # For Hydro, add special consideration for stable pools
                is_stable_pool = pool.get("pool_type", "") == "Stable"

                # Add metrics to pool data
                opportunity = {
                    **pool,
                    "apy_volatility": round(apy_volatility, 4),
                    "risk_adjusted_return": round(risk_adjusted_return, 2),
                    "is_stable_pool": is_stable_pool,
                    "stability_bonus": 0.5
                    if is_stable_pool
                    else 0,  # Bonus for stable pools in ranking
                    "analysis_timestamp": datetime.now().isoformat(),
                }

                opportunities.append(opportunity)

            # Sort by adjusted ranking that considers stability for Hydro
            opportunities.sort(
                key=lambda x: x.get("risk_adjusted_return", 0)
                + x.get("stability_bonus", 0),
                reverse=True,
            )

            # Get protocol info
            protocol_info = await self.adapter.get_protocol_info()

            # Organize opportunities by pool type (a Hydro-specific feature)
            stable_opportunities = [o for o in opportunities if o.get("is_stable_pool")]
            weighted_opportunities = [
                o for o in opportunities if not o.get("is_stable_pool")
            ]

            return {
                "protocol": {
                    "id": self.protocol_id,
                    "name": protocol_info.get("name", "Hydro Protocol"),
                    "total_tvl": sum(pool.get("tvl", 0) for pool in pools),
                    "pool_count": len(pools),
                    "specialization": protocol_info.get(
                        "specialization", "Stablecoin liquidity"
                    ),
                },
                "opportunities": opportunities,
                "stable_pools": stable_opportunities,
                "weighted_pools": weighted_opportunities,
                "top_opportunity": opportunities[0] if opportunities else None,
                "top_stable_opportunity": stable_opportunities[0]
                if stable_opportunities
                else None,
                "analysis_timestamp": datetime.now().isoformat(),
            }

        except Exception as e:
            self.logger.error(f"Error analyzing opportunities: {e}")
            return {
                "protocol": {
                    "id": self.protocol_id,
                    "name": "Hydro Protocol",
                    "error": str(e),
                },
                "opportunities": [],
                "analysis_timestamp": datetime.now().isoformat(),
            }

    async def get_risk_assessment(self) -> Dict[str, Any]:
        """
        Get risk assessment for Hydro protocol

        Returns:
            Risk assessment metrics
        """
        try:
            # Get protocol risk from adapter
            protocol_risk = await self.adapter.estimate_protocol_risk()

            # Get pools to assess pool-specific risks
            pools = await self.adapter.get_pools()

            # Calculate average risk metrics - Hydro has lower risks for stable pools
            stable_pools = [p for p in pools if p.get("pool_type", "") == "Stable"]
            weighted_pools = [p for p in pools if p.get("pool_type", "") != "Stable"]

            avg_stable_risk = (
                sum(pool.get("risk_score", 3.0) for pool in stable_pools)
                / len(stable_pools)
                if stable_pools
                else 3.0
            )
            avg_weighted_risk = (
                sum(pool.get("risk_score", 5.0) for pool in weighted_pools)
                / len(weighted_pools)
                if weighted_pools
                else 5.0
            )

            # Calculate overall pool risk with weighted average
            stable_weight = len(stable_pools) / len(pools) if pools else 0.5
            avg_pool_risk = (avg_stable_risk * stable_weight) + (
                avg_weighted_risk * (1 - stable_weight)
            )

            # Get TVL for size-based risk
            total_tvl = sum(pool.get("tvl", 0) for pool in pools)
            tvl_risk = max(1.0, 10.0 - min(9.0, total_tvl / 1_000_000 * 0.5))

            # For Hydro, calculate stablecoin ratio as a risk factor
            total_tvl = sum(pool.get("tvl", 0) for pool in pools)
            stable_tvl = sum(pool.get("tvl", 0) for pool in stable_pools)
            stablecoin_ratio = stable_tvl / total_tvl if total_tvl > 0 else 0.5

            # Higher stablecoin ratio = lower risk
            stablecoin_risk = 7.0 - (stablecoin_ratio * 4.0)

            # Combine risk factors into overall risk
            weights = {"protocol": 0.25, "pools": 0.25, "tvl": 0.25, "stablecoin": 0.25}

            overall_risk = (
                protocol_risk * weights["protocol"]
                + avg_pool_risk * weights["pools"]
                + tvl_risk * weights["tvl"]
                + stablecoin_risk * weights["stablecoin"]
            )

            return {
                "protocol_id": self.protocol_id,
                "overall_risk": round(overall_risk, 1),
                "protocol_risk": protocol_risk,
                "average_pool_risk": round(avg_pool_risk, 1),
                "stable_pool_risk": round(avg_stable_risk, 1),
                "weighted_pool_risk": round(avg_weighted_risk, 1),
                "tvl_risk": round(tvl_risk, 1),
                "stablecoin_ratio": round(stablecoin_ratio, 2),
                "stablecoin_risk": round(stablecoin_risk, 1),
                "assessment_timestamp": datetime.now().isoformat(),
            }

        except Exception as e:
            self.logger.error(f"Error getting risk assessment: {e}")
            return {
                "protocol_id": self.protocol_id,
                "overall_risk": 4.0,  # Default slightly below moderate risk for Hydro
                "error": str(e),
                "assessment_timestamp": datetime.now().isoformat(),
            }

    async def get_recommended_pools(
        self, risk_profile: str = "moderate"
    ) -> List[Dict[str, Any]]:
        """
        Get recommended Hydro pools based on risk profile

        For Hydro, we emphasize stable pools for conservative profiles

        Args:
            risk_profile: Risk profile (conservative, moderate, aggressive)

        Returns:
            List of recommended pools with rationale
        """
        try:
            # Get opportunities analysis
            analysis = await self.analyze_opportunities()
            opportunities = analysis.get("opportunities", [])
            stable_pools = analysis.get("stable_pools", [])
            weighted_pools = analysis.get("weighted_pools", [])

            if not opportunities:
                return []

            # Define risk thresholds based on profile
            risk_thresholds = {
                "conservative": 3.5,  # Lower threshold for Hydro
                "moderate": 5.0,
                "aggressive": 7.0,
            }

            risk_threshold = risk_thresholds.get(risk_profile, 5.0)

            # For Hydro, we have special handling based on risk profile
            if risk_profile == "conservative":
                # For conservative profiles, strongly prefer stable pools
                eligible_pools = [
                    pool
                    for pool in stable_pools
                    if pool.get("risk_score", 5.0) <= risk_threshold
                ]

                # If we don't have enough stable pools, add some safe weighted pools
                if len(eligible_pools) < 2:
                    safe_weighted = [
                        pool
                        for pool in weighted_pools
                        if pool.get("risk_score", 5.0) <= risk_threshold
                    ]
                    eligible_pools.extend(safe_weighted[: 2 - len(eligible_pools)])

                # Sort by risk score (ascending)
                eligible_pools.sort(key=lambda x: x.get("risk_score", 10))

            elif risk_profile == "aggressive":
                # For aggressive, prioritize weighted pools by APY
                eligible_pools = [
                    pool
                    for pool in weighted_pools
                    if pool.get("risk_score", 5.0) <= risk_threshold
                ]

                # Sort by APY (descending)
                eligible_pools.sort(key=lambda x: x.get("apy", 0), reverse=True)

            else:  # moderate
                # For moderate, balance between stable and weighted based on risk-adjusted return
                eligible_pools = [
                    pool
                    for pool in opportunities
                    if pool.get("risk_score", 5.0) <= risk_threshold
                ]

                # Sort by risk-adjusted return
                eligible_pools.sort(
                    key=lambda x: x.get("risk_adjusted_return", 0), reverse=True
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
                    is_stable = rec.get("is_stable_pool", False)

                    if risk_profile == "conservative":
                        if is_stable:
                            rec["recommendation_note"] = (
                                f"Low risk stable pool with {rec.get('apy', 0)}% APY"
                            )
                        else:
                            rec["recommendation_note"] = (
                                f"Relatively safe weighted pool with good stability"
                            )
                    elif risk_profile == "aggressive":
                        if is_stable:
                            rec["recommendation_note"] = (
                                f"Stable pool providing reliable but modest returns"
                            )
                        else:
                            rec["recommendation_note"] = (
                                f"Higher yield weighted pool with acceptable risk level"
                            )
                    else:
                        if is_stable:
                            rec["recommendation_note"] = (
                                f"Stable pool with solid risk-adjusted returns"
                            )
                        else:
                            rec["recommendation_note"] = (
                                f"Balanced risk-reward profile in weighted pool"
                            )

            return recommendations

        except Exception as e:
            self.logger.error(f"Error getting recommended pools: {e}")
            return []

    async def estimate_correlation(self, other_protocol_id: str) -> float:
        """
        Estimate correlation between Hydro and another protocol

        Args:
            other_protocol_id: ID of the other protocol

        Returns:
            Correlation coefficient (-1 to 1)
        """
        # Hydro tends to have lower correlations due to focus on stablecoins

        # Placeholder correlation estimates based on protocol characteristics
        correlations = {
            "helix": 0.6,  # Moderate correlation with Helix
            "neptune": 0.4,  # Lower correlation with Neptune
        }

        return correlations.get(other_protocol_id, 0.5)  # Default moderate correlation

    async def get_historical_performance(self, days: int = 30) -> Dict[str, Any]:
        """
        Get historical performance metrics for Hydro

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

            # Separate stable and weighted pools for Hydro-specific analytics
            stable_pools = [
                p["id"] for p in pools if p.get("pool_type", "") == "Stable"
            ]
            weighted_pools = [
                p["id"] for p in pools if p.get("pool_type", "") != "Stable"
            ]

            # Calculate protocol-level performance metrics
            daily_metrics = []
            stable_metrics = []
            weighted_metrics = []

            for i in range(min(days, len(next(iter(historical_data.values()), [])))):
                day_data = {"date": "", "total_tvl": 0, "avg_apy": 0, "volume": 0}

                stable_data = {"date": "", "total_tvl": 0, "avg_apy": 0, "count": 0}

                weighted_data = {"date": "", "total_tvl": 0, "avg_apy": 0, "count": 0}

                # Collect metrics for this day across all pools
                tvl_values = []
                apy_values = []
                volume_values = []

                stable_tvls = []
                stable_apys = []

                weighted_tvls = []
                weighted_apys = []

                for pool_id, history in historical_data.items():
                    if i < len(history):
                        day = history[i]
                        # Set date from first pool (they should all be the same)
                        if not day_data["date"]:
                            day_data["date"] = day.get("date", "")
                            stable_data["date"] = day.get("date", "")
                            weighted_data["date"] = day.get("date", "")

                        tvl = day.get("tvl", 0)
                        apy = day.get("apy", 0)
                        volume = day.get("volume_24h", 0)

                        tvl_values.append(tvl)
                        apy_values.append(apy)
                        volume_values.append(volume)

                        # Separate stable and weighted pool metrics
                        if pool_id in stable_pools:
                            stable_tvls.append(tvl)
                            stable_apys.append(apy)
                            stable_data["count"] += 1
                        elif pool_id in weighted_pools:
                            weighted_tvls.append(tvl)
                            weighted_apys.append(apy)
                            weighted_data["count"] += 1

                # Calculate aggregates
                day_data["total_tvl"] = sum(tvl_values)
                day_data["avg_apy"] = (
                    sum(apy_values) / len(apy_values) if apy_values else 0
                )
                day_data["volume"] = sum(volume_values)

                # Stable pool metrics
                stable_data["total_tvl"] = sum(stable_tvls)
                stable_data["avg_apy"] = (
                    sum(stable_apys) / len(stable_apys) if stable_apys else 0
                )

                # Weighted pool metrics
                weighted_data["total_tvl"] = sum(weighted_tvls)
                weighted_data["avg_apy"] = (
                    sum(weighted_apys) / len(weighted_apys) if weighted_apys else 0
                )

                daily_metrics.append(day_data)
                stable_metrics.append(stable_data)
                weighted_metrics.append(weighted_data)

            return {
                "protocol_id": self.protocol_id,
                "days": days,
                "pools": historical_data,
                "protocol_metrics": daily_metrics,
                "stable_pool_metrics": stable_metrics,
                "weighted_pool_metrics": weighted_metrics,
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

    async def analyze_stable_pool_efficiency(self) -> Dict[str, Any]:
        """
        Analyze the efficiency of Hydro's stable pools

        This is a Hydro-specific analysis since stable pools are a key feature

        Returns:
            Stable pool efficiency metrics
        """
        try:
            # Get all pools
            pools = await self.adapter.get_pools()

            # Filter for stable pools
            stable_pools = [p for p in pools if p.get("pool_type", "") == "Stable"]

            if not stable_pools:
                return {
                    "status": "No stable pools found",
                    "timestamp": datetime.now().isoformat(),
                }

            # Analyze each stable pool
            pool_metrics = []
            for pool in stable_pools:
                pool_id = pool["id"]

                # Get pool parameters
                parameters = await self.adapter.get_pool_parameters(pool_id)

                # Get recent historical data to assess efficiency
                history = await self.adapter.get_historical_data(pool_id, days=7)

                # Calculate metrics
                avg_slippage = (
                    0.0001  # Mock value - would be calculated from actual trade data
                )
                price_stability = 0.9995  # How close to 1:1 the stable tokens trade

                # Calculate efficiency score (higher is better)
                efficiency_score = (
                    (1.0 - avg_slippage) * 0.4  # Low slippage is good
                    + price_stability * 0.4  # Price stability is good
                    + min(1.0, pool.get("tvl", 0) / 1_000_000)
                    * 0.2  # Higher TVL is good
                ) * 10  # Scale to 0-10

                pool_metrics.append(
                    {
                        "pool_id": pool_id,
                        "name": pool.get("name", ""),
                        "apy": pool.get("apy", 0),
                        "tvl": pool.get("tvl", 0),
                        "avg_slippage": avg_slippage,
                        "price_stability": price_stability,
                        "efficiency_score": round(efficiency_score, 2),
                        "amp_parameter": parameters.get(
                            "amp", 100
                        ),  # Amplification parameter
                    }
                )

            # Sort by efficiency score
            pool_metrics.sort(key=lambda x: x.get("efficiency_score", 0), reverse=True)

            # Calculate protocol-level metrics
            avg_efficiency = sum(
                p.get("efficiency_score", 0) for p in pool_metrics
            ) / len(pool_metrics)
            total_stable_tvl = sum(p.get("tvl", 0) for p in pool_metrics)

            return {
                "protocol_id": self.protocol_id,
                "stable_pool_count": len(stable_pools),
                "total_stable_tvl": total_stable_tvl,
                "average_efficiency_score": round(avg_efficiency, 2),
                "top_pool": pool_metrics[0] if pool_metrics else None,
                "pool_metrics": pool_metrics,
                "timestamp": datetime.now().isoformat(),
            }

        except Exception as e:
            self.logger.error(f"Error analyzing stable pool efficiency: {e}")
            return {
                "protocol_id": self.protocol_id,
                "error": str(e),
                "timestamp": datetime.now().isoformat(),
            }
