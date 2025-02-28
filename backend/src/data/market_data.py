"""
Market Data Collection for AstroBalance

This module provides market data collection for various protocols on Injective.
During development, it uses mock data that mimics real protocol metrics.
In production, this would be replaced with actual API calls to protocol endpoints.
"""

import yaml
import os
import json
import asyncio
import logging
import random
from datetime import datetime, timedelta
from typing import Dict, List, Any, Optional, Union
import numpy as np

logger = logging.getLogger("astrobalance.data")


class ProtocolData:
    """Base class for protocol data"""

    def __init__(self, protocol_id: str, config_path: str = None):
        """
        Initialize protocol data collector

        Args:
            protocol_id: Protocol identifier
            config_path: Path to config file (optional)
        """
        self.protocol_id = protocol_id
        self.logger = logging.getLogger(f"astrobalance.data.{protocol_id}")

        # Load configuration
        self.config = self._load_config(config_path)

        # Initialize data storage
        self.pools = self._initialize_mock_pools()

    def _load_config(self, config_path: str = None) -> Dict[str, Any]:
        """
        Load protocol configuration

        Args:
            config_path: Path to config file

        Returns:
            Configuration dictionary
        """
        default_path = os.path.join("config", f"{self.protocol_id}.yaml")
        path = config_path or default_path

        try:
            if os.path.exists(path):
                with open(path, "r") as f:
                    return yaml.safe_load(f)
            else:
                self.logger.warning(f"Config file not found: {path}, using defaults")
                return {}
        except Exception as e:
            self.logger.error(f"Error loading config: {e}")
            return {}

    def _initialize_mock_pools(self) -> List[Dict[str, Any]]:
        """
        Initialize mock pool data

        Returns:
            List of mock pools
        """
        # To be implemented by protocol-specific subclasses
        return []

    async def get_pools(self) -> List[Dict[str, Any]]:
        """
        Get pool data

        Returns:
            List of pools with their data
        """
        # Add some random fluctuations to APY for realism
        for pool in self.pools:
            # Random fluctuation of ±5%
            fluctuation = 1 + (random.random() - 0.5) * 0.1
            pool["apy"] = round(pool["base_apy"] * fluctuation, 2)

            # Small random fluctuation in TVL
            tvl_fluctuation = 1 + (random.random() - 0.5) * 0.05
            pool["tvl"] = round(pool["base_tvl"] * tvl_fluctuation, 0)

        return self.pools

    async def get_tvl(self) -> Dict[str, float]:
        """
        Get TVL for each pool

        Returns:
            Dictionary mapping pool IDs to TVL
        """
        pools = await self.get_pools()
        return {pool["id"]: pool["tvl"] for pool in pools}

    async def get_historical_data(
        self, days: int = 30
    ) -> Dict[str, List[Dict[str, Any]]]:
        """
        Get historical data for pools

        Args:
            days: Number of days of history

        Returns:
            Dictionary mapping pool IDs to historical data
        """
        pools = await self.get_pools()
        result = {}

        for pool in pools:
            pool_id = pool["id"]
            history = []

            # Generate synthetic history
            base_apy = pool["base_apy"]
            base_tvl = pool["base_tvl"]

            # Use pool name hash for deterministic randomness
            seed = hash(pool_id) % 10000
            rng = random.Random(seed)

            for i in range(days):
                date = datetime.now() - timedelta(days=i)

                # Generate APY with trend and noise
                # More volatile pools have more noise
                volatility = pool.get("volatility", 0.05)
                noise_scale = volatility * base_apy
                noise = (rng.random() - 0.5) * 2 * noise_scale

                # Add a slight trend
                trend = -0.001 * i * base_apy  # Slight downtrend over time

                daily_apy = max(0.1, base_apy + trend + noise)

                # Generate TVL with growth and noise
                tvl_growth = 1 - (0.0005 * i)  # Slight growth going backwards in time
                tvl_noise = (rng.random() - 0.5) * 0.03 * base_tvl
                daily_tvl = max(1000, base_tvl * tvl_growth + tvl_noise)

                history.append(
                    {
                        "date": date.isoformat(),
                        "apy": round(daily_apy, 2),
                        "tvl": round(daily_tvl, 0),
                        "volume_24h": round(
                            daily_tvl * rng.uniform(0.05, 0.15), 0
                        ),  # 5-15% daily volume
                    }
                )

            result[pool_id] = history

        return result


class HelixData(ProtocolData):
    """Data collector for Helix protocol"""

    def __init__(self, config_path: str = None):
        """Initialize Helix data collector"""
        super().__init__("helix", config_path)

    def _initialize_mock_pools(self) -> List[Dict[str, Any]]:
        """Initialize mock Helix pools"""
        return [
            {
                "id": "helix_usdc_inj",
                "name": "USDC-INJ LP",
                "base_apy": 28.5,
                "apy": 28.5,
                "base_tvl": 1850000,
                "tvl": 1850000,
                "pair": ["USDC", "INJ"],
                "provider": "Helix",
                "volatility": 0.08,
                "risk_score": 6.2,
            },
            {
                "id": "helix_usdc_atom",
                "name": "USDC-ATOM LP",
                "base_apy": 22.3,
                "apy": 22.3,
                "base_tvl": 920000,
                "tvl": 920000,
                "pair": ["USDC", "ATOM"],
                "provider": "Helix",
                "volatility": 0.06,
                "risk_score": 5.7,
            },
            {
                "id": "helix_usdc_usdt",
                "name": "USDC-USDT LP",
                "base_apy": 12.8,
                "apy": 12.8,
                "base_tvl": 4300000,
                "tvl": 4300000,
                "pair": ["USDC", "USDT"],
                "provider": "Helix",
                "volatility": 0.03,
                "risk_score": 2.8,
            },
        ]


class HydroData(ProtocolData):
    """Data collector for Hydro protocol"""

    def __init__(self, config_path: str = None):
        """Initialize Hydro data collector"""
        super().__init__("hydro", config_path)

    def _initialize_mock_pools(self) -> List[Dict[str, Any]]:
        """Initialize mock Hydro pools"""
        return [
            {
                "id": "hydro_usdc_usdt",
                "name": "USDC-USDT Stable Pool",
                "base_apy": 8.2,
                "apy": 8.2,
                "base_tvl": 5600000,
                "tvl": 5600000,
                "pair": ["USDC", "USDT"],
                "provider": "Hydro",
                "volatility": 0.02,
                "risk_score": 2.1,
            },
            {
                "id": "hydro_usdc_dai",
                "name": "USDC-DAI Stable Pool",
                "base_apy": 7.9,
                "apy": 7.9,
                "base_tvl": 2900000,
                "tvl": 2900000,
                "pair": ["USDC", "DAI"],
                "provider": "Hydro",
                "volatility": 0.02,
                "risk_score": 2.3,
            },
            {
                "id": "hydro_inj_atom",
                "name": "INJ-ATOM Trading Pool",
                "base_apy": 32.5,
                "apy": 32.5,
                "base_tvl": 840000,
                "tvl": 840000,
                "pair": ["INJ", "ATOM"],
                "provider": "Hydro",
                "volatility": 0.09,
                "risk_score": 7.1,
            },
        ]


class NeptuneData(ProtocolData):
    """Data collector for Neptune protocol"""

    def __init__(self, config_path: str = None):
        """Initialize Neptune data collector"""
        super().__init__("neptune", config_path)

    def _initialize_mock_pools(self) -> List[Dict[str, Any]]:
        """Initialize mock Neptune pools"""
        return [
            {
                "id": "neptune_usdc_atom",
                "name": "USDC-ATOM Yield Pool",
                "base_apy": 24.8,
                "apy": 24.8,
                "base_tvl": 1250000,
                "tvl": 1250000,
                "pair": ["USDC", "ATOM"],
                "provider": "Neptune",
                "volatility": 0.07,
                "risk_score": 5.9,
            },
            {
                "id": "neptune_usdc_inj",
                "name": "USDC-INJ Yield Pool",
                "base_apy": 29.2,
                "apy": 29.2,
                "base_tvl": 1650000,
                "tvl": 1650000,
                "pair": ["USDC", "INJ"],
                "provider": "Neptune",
                "volatility": 0.09,
                "risk_score": 6.8,
            },
        ]


class MarketDataCollector:
    """
    Collects and aggregates data from all protocols

    This class provides a unified interface to collect data from
    all supported protocols and aggregate it for analysis.
    """

    def __init__(self):
        """Initialize market data collector"""
        self.logger = logging.getLogger("astrobalance.data.collector")

        # Initialize protocol data collectors
        self.protocols = {
            "helix": HelixData(),
            "hydro": HydroData(),
            "neptune": NeptuneData(),
        }

    async def get_all_pools(self) -> List[Dict[str, Any]]:
        """
        Get pool data from all protocols

        Returns:
            Combined list of pools from all protocols
        """
        all_pools = []
        for protocol_id, protocol in self.protocols.items():
            try:
                pools = await protocol.get_pools()
                all_pools.extend(pools)
            except Exception as e:
                self.logger.error(f"Error getting pools from {protocol_id}: {e}")

        return all_pools

    async def get_all_tvl(self) -> Dict[str, float]:
        """
        Get TVL data from all protocols

        Returns:
            Combined TVL data from all protocols
        """
        all_tvl = {}
        for protocol_id, protocol in self.protocols.items():
            try:
                tvl = await protocol.get_tvl()
                all_tvl.update(tvl)
            except Exception as e:
                self.logger.error(f"Error getting TVL from {protocol_id}: {e}")

        return all_tvl

    async def get_protocol_stats(self, protocol_id: str = None) -> Dict[str, Any]:
        """
        Get detailed statistics for protocols

        Args:
            protocol_id: Protocol ID or None for all protocols

        Returns:
            Protocol statistics
        """
        result = {}

        if protocol_id and protocol_id != "all":
            if protocol_id not in self.protocols:
                self.logger.error(f"Unknown protocol: {protocol_id}")
                return {"error": f"Unknown protocol: {protocol_id}"}

            protocols_to_check = [protocol_id]
        else:
            protocols_to_check = list(self.protocols.keys())

        for proto_id in protocols_to_check:
            protocol = self.protocols[proto_id]

            try:
                # Get pools and calculate protocol stats
                pools = await protocol.get_pools()

                # Calculate aggregate metrics
                total_tvl = sum(pool["tvl"] for pool in pools)
                avg_apy = (
                    sum(pool["apy"] for pool in pools) / len(pools) if pools else 0
                )
                avg_risk = (
                    sum(pool.get("risk_score", 5) for pool in pools) / len(pools)
                    if pools
                    else 5
                )

                # Store in result
                result[proto_id] = {
                    "name": proto_id.capitalize(),
                    "total_tvl": total_tvl,
                    "average_apy": avg_apy,
                    "risk_score": avg_risk,
                    "pools": pools,
                    "pool_count": len(pools),
                }

            except Exception as e:
                self.logger.error(f"Error getting stats for {proto_id}: {e}")
                result[proto_id] = {"error": str(e)}

        return result

    async def get_all_historical_data(
        self, days: int = 30
    ) -> Dict[str, Dict[str, List[Dict[str, Any]]]]:
        """
        Get historical data from all protocols

        Args:
            days: Number of days of history

        Returns:
            Historical data organized by protocol and pool
        """
        result = {}
        for protocol_id, protocol in self.protocols.items():
            try:
                history = await protocol.get_historical_data(days)
                result[protocol_id] = history
            except Exception as e:
                self.logger.error(
                    f"Error getting historical data from {protocol_id}: {e}"
                )

        return result


# Simple test function to verify functionality
async def test_market_data():
    collector = MarketDataCollector()

    print("Testing pool data collection...")
    pools = await collector.get_all_pools()
    print(f"Found {len(pools)} pools across all protocols")

    print("\nTesting TVL data collection...")
    tvl = await collector.get_all_tvl()
    print(f"Total TVL across all protocols: ${sum(tvl.values()):,.2f}")

    print("\nTesting protocol stats...")
    stats = await collector.get_protocol_stats()
    for protocol_id, protocol_stats in stats.items():
        print(
            f"- {protocol_id}: {protocol_stats['pool_count']} pools, ${protocol_stats['total_tvl']:,.2f} TVL"
        )

    print("\nTesting historical data...")
    history = await collector.get_all_historical_data(days=7)
    for protocol_id, protocol_history in history.items():
        for pool_id, pool_history in protocol_history.items():
            print(f"- {protocol_id}/{pool_id}: {len(pool_history)} days of history")
            print(
                f"  Latest APY: {pool_history[0]['apy']}%, TVL: ${pool_history[0]['tvl']:,.2f}"
            )

    return pools, tvl, stats, history


if __name__ == "__main__":
    asyncio.run(test_market_data())
