"""
Base Protocol Adapter Interface

This module defines the base interface for protocol adapters,
which are responsible for communicating with DeFi protocols
and retrieving relevant data.
"""

import logging
from abc import ABC, abstractmethod
from typing import Dict, List, Any, Optional, Tuple


class BaseAdapter(ABC):
    """
    Base class for all protocol adapters

    Protocol adapters are responsible for:
    1. Retrieving pool/asset data from protocols
    2. Parsing protocol-specific responses
    3. Providing a standardized interface for protocol data
    """

    def __init__(self, protocol_id: str, config_path: Optional[str] = None):
        """
        Initialize protocol adapter

        Args:
            protocol_id: Unique identifier for the protocol
            config_path: Path to configuration file (optional)
        """
        self.protocol_id = protocol_id
        self.logger = logging.getLogger(f"astrobalance.adapter.{protocol_id}")

    @abstractmethod
    async def get_pools(self) -> List[Dict[str, Any]]:
        """
        Get all liquidity pools from the protocol

        Returns:
            List of pool data dictionaries
        """
        pass

    @abstractmethod
    async def get_pool_details(self, pool_id: str) -> Dict[str, Any]:
        """
        Get detailed information about a specific pool

        Args:
            pool_id: Identifier for the pool

        Returns:
            Detailed pool information
        """
        pass

    @abstractmethod
    async def get_apy(self, pool_id: str) -> float:
        """
        Get the current APY for a specific pool

        Args:
            pool_id: Identifier for the pool

        Returns:
            Current APY as a percentage
        """
        pass

    @abstractmethod
    async def get_tvl(self, pool_id: Optional[str] = None) -> Dict[str, float]:
        """
        Get total value locked for pools

        Args:
            pool_id: If provided, get TVL for specific pool, otherwise all pools

        Returns:
            Dictionary mapping pool IDs to TVL values
        """
        pass

    @abstractmethod
    async def get_historical_data(
        self, pool_id: str, days: int = 30
    ) -> List[Dict[str, Any]]:
        """
        Get historical performance data for a pool

        Args:
            pool_id: Identifier for the pool
            days: Number of days of historical data to retrieve

        Returns:
            List of historical data points
        """
        pass

    @abstractmethod
    async def get_assets(self, pool_id: str) -> List[Dict[str, Any]]:
        """
        Get assets in a specific pool

        Args:
            pool_id: Identifier for the pool

        Returns:
            List of assets in the pool with details
        """
        pass

    @abstractmethod
    async def get_protocol_info(self) -> Dict[str, Any]:
        """
        Get general information about the protocol

        Returns:
            Protocol information including name, description, etc.
        """
        pass

    async def is_available(self) -> bool:
        """
        Check if the protocol is available/online

        Returns:
            True if the protocol is available, False otherwise
        """
        try:
            await self.get_protocol_info()
            return True
        except Exception as e:
            self.logger.warning(f"Protocol {self.protocol_id} is not available: {e}")
            return False

    async def estimate_protocol_risk(self) -> float:
        """
        Estimate overall protocol risk on a scale of 1-10

        Returns:
            Risk score (1 = lowest risk, 10 = highest risk)
        """
        # Default implementation based on protocol history and size
        # Protocols should override this for more specific risk calculations
        try:
            tvl_data = await self.get_tvl()
            total_tvl = sum(tvl_data.values())

            # Larger TVL = lower base risk (simple heuristic)
            tvl_factor = max(1.0, 10.0 - min(9.0, total_tvl / 1_000_000 * 0.5))

            # Default moderate risk score if no better data is available
            return tvl_factor
        except Exception as e:
            self.logger.warning(f"Error estimating protocol risk: {e}")
            return 5.0  # Default moderate risk
