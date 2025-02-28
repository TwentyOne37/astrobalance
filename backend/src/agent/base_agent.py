"""
Base Protocol Agent

This module defines the base interface for protocol agents,
which are responsible for analyzing protocol data and making
yield optimization recommendations.
"""

import logging
from abc import ABC, abstractmethod
from typing import Dict, List, Any, Optional, Tuple


class BaseAgent(ABC):
    """
    Base class for all protocol agents

    Protocol agents are responsible for:
    1. Analyzing protocol data
    2. Assessing risk and return characteristics
    3. Making yield optimization recommendations
    4. Providing protocol-specific insights
    """

    def __init__(self, protocol_id: str):
        """
        Initialize protocol agent

        Args:
            protocol_id: Unique identifier for the protocol
        """
        self.protocol_id = protocol_id
        self.logger = logging.getLogger(f"astrobalance.agent.{protocol_id}")

    @abstractmethod
    async def analyze_opportunities(self) -> Dict[str, Any]:
        """
        Analyze yield opportunities in the protocol

        Returns:
            Dictionary of opportunities with analysis
        """
        pass

    @abstractmethod
    async def get_risk_assessment(self) -> Dict[str, Any]:
        """
        Get risk assessment for the protocol

        Returns:
            Risk assessment metrics
        """
        pass

    @abstractmethod
    async def get_recommended_pools(
        self, risk_profile: str = "moderate"
    ) -> List[Dict[str, Any]]:
        """
        Get recommended pools based on risk profile

        Args:
            risk_profile: Risk profile (conservative, moderate, aggressive)

        Returns:
            List of recommended pools with rationale
        """
        pass

    @abstractmethod
    async def estimate_correlation(self, other_protocol_id: str) -> float:
        """
        Estimate correlation with another protocol

        Args:
            other_protocol_id: ID of the other protocol

        Returns:
            Correlation coefficient (-1 to 1)
        """
        pass

    @abstractmethod
    async def get_historical_performance(self, days: int = 30) -> Dict[str, Any]:
        """
        Get historical performance metrics

        Args:
            days: Number of days of history

        Returns:
            Historical performance metrics
        """
        pass

    async def get_protocol_health(self) -> Dict[str, Any]:
        """
        Get overall protocol health metrics

        Returns:
            Protocol health assessment
        """
        try:
            # Get risk assessment
            risk = await self.get_risk_assessment()

            # Get historical performance for volatility
            history = await self.get_historical_performance(days=30)

            # Calculate health metrics
            tvl_stability = self._calculate_tvl_stability(history)
            apy_stability = self._calculate_apy_stability(history)

            return {
                "protocol_id": self.protocol_id,
                "risk_score": risk.get("overall_risk", 5.0),
                "tvl_stability": tvl_stability,
                "apy_stability": apy_stability,
                "health_score": self._calculate_health_score(
                    risk.get("overall_risk", 5.0), tvl_stability, apy_stability
                ),
                "analysis_timestamp": history.get("timestamp", ""),
            }
        except Exception as e:
            self.logger.error(f"Error getting protocol health: {e}")
            return {
                "protocol_id": self.protocol_id,
                "health_score": 5.0,  # Default moderate health
                "error": str(e),
            }

    def _calculate_tvl_stability(self, history: Dict[str, Any]) -> float:
        """Calculate TVL stability from historical data"""
        try:
            # Extract TVL values from history
            tvl_values = []
            for pool_id, pool_history in history.get("pools", {}).items():
                for day in pool_history:
                    if "tvl" in day:
                        tvl_values.append(day["tvl"])

            if not tvl_values:
                return 5.0  # Default moderate stability

            # Calculate coefficient of variation (lower = more stable)
            import numpy as np

            tvl_array = np.array(tvl_values)
            cv = np.std(tvl_array) / np.mean(tvl_array) if np.mean(tvl_array) > 0 else 0

            # Convert to stability score (10 = most stable)
            stability = 10 - min(9, cv * 100)

            return round(stability, 1)
        except Exception as e:
            self.logger.error(f"Error calculating TVL stability: {e}")
            return 5.0  # Default moderate stability

    def _calculate_apy_stability(self, history: Dict[str, Any]) -> float:
        """Calculate APY stability from historical data"""
        try:
            # Extract APY values from history
            apy_values = []
            for pool_id, pool_history in history.get("pools", {}).items():
                for day in pool_history:
                    if "apy" in day:
                        apy_values.append(day["apy"])

            if not apy_values:
                return 5.0  # Default moderate stability

            # Calculate coefficient of variation (lower = more stable)
            import numpy as np

            apy_array = np.array(apy_values)
            cv = np.std(apy_array) / np.mean(apy_array) if np.mean(apy_array) > 0 else 0

            # Convert to stability score (10 = most stable)
            stability = 10 - min(9, cv * 50)  # APY can be more volatile

            return round(stability, 1)
        except Exception as e:
            self.logger.error(f"Error calculating APY stability: {e}")
            return 5.0  # Default moderate stability

    def _calculate_health_score(
        self, risk: float, tvl_stability: float, apy_stability: float
    ) -> float:
        """Calculate overall health score"""
        # Invert risk (10 - risk) so higher is better
        risk_factor = 10 - risk

        # Weight factors for health score
        weights = {"risk": 0.4, "tvl_stability": 0.3, "apy_stability": 0.3}

        # Calculate weighted score
        health_score = (
            risk_factor * weights["risk"]
            + tvl_stability * weights["tvl_stability"]
            + apy_stability * weights["apy_stability"]
        )

        return round(health_score, 1)
