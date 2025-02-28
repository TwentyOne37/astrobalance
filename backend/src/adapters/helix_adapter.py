"""
Helix Protocol Adapter

This module implements the adapter for the Helix protocol on Injective.
It handles all communication with the Helix protocol endpoints.
"""

import yaml
import aiohttp
import os
import json
import logging
from typing import Dict, List, Any, Optional
from datetime import datetime, timedelta

from .base_adapter import BaseAdapter
from src.data.market_data import HelixData


class HelixAdapter(BaseAdapter):
    """Adapter for Helix protocol"""

    def __init__(self, config_path: Optional[str] = None):
        """
        Initialize Helix adapter

        Args:
            config_path: Path to configuration file (optional)
        """
        super().__init__("helix", config_path)
        self.config = self._load_config(config_path)
        self.api_url = self.config.get("api_url", "https://helix.injective.network/api")

        # Initialize the data provider, with fallback to mock data
        self.data_provider = HelixData(config_path)
        self.use_mock_data = self.config.get("enable_mock_data", True)

    def _load_config(self, config_path: Optional[str] = None) -> Dict[str, Any]:
        """
        Load protocol configuration

        Args:
            config_path: Path to configuration file

        Returns:
            Configuration dictionary
        """
        default_path = os.path.join("config", "helix.yaml")
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

    async def _api_request(
        self, endpoint: str, params: Optional[Dict[str, Any]] = None
    ) -> Dict[str, Any]:
        """
        Make an API request to the Helix protocol

        Args:
            endpoint: API endpoint
            params: Query parameters

        Returns:
            API response
        """
        url = f"{self.api_url}/{endpoint}"

        try:
            async with aiohttp.ClientSession() as session:
                async with session.get(url, params=params) as response:
                    if response.status != 200:
                        self.logger.error(
                            f"API request failed: {response.status} - {await response.text()}"
                        )
                        raise Exception(f"API request failed: {response.status}")

                    data = await response.json()
                    return data
        except Exception as e:
            self.logger.error(f"Error making API request: {e}")
            if self.use_mock_data:
                self.logger.info("Falling back to mock data")
                # Simulate API response based on mock data
                return {"error": str(e), "using_mock_data": True}
            raise

    async def get_pools(self) -> List[Dict[str, Any]]:
        """
        Get all liquidity pools from Helix

        Returns:
            List of pool data dictionaries
        """
        try:
            # Try to get real data from API
            if not self.use_mock_data:
                data = await self._api_request("pools")
                if "error" not in data:
                    # Process API response
                    return self._process_pools_response(data)

            # Fall back to mock data
            pools = await self.data_provider.get_pools()
            return pools
        except Exception as e:
            self.logger.error(f"Error getting pools: {e}")

            # Last resort fallback - if even mock data fails
            pools = await self.data_provider.get_pools() if self.use_mock_data else []
            return pools

    def _process_pools_response(self, data: Dict[str, Any]) -> List[Dict[str, Any]]:
        """
        Process pools API response

        Args:
            data: API response data

        Returns:
            Processed pool data
        """
        # This would be implemented based on actual Helix API response format
        # For now, this is a placeholder
        pools = []
        for pool_data in data.get("pools", []):
            pools.append(
                {
                    "id": pool_data.get("id", ""),
                    "name": pool_data.get("name", ""),
                    "apy": float(pool_data.get("apy", 0)),
                    "tvl": float(pool_data.get("tvl", 0)),
                    "pair": pool_data.get("tokens", []),
                    "provider": "Helix",
                    "volatility": float(pool_data.get("volatility", 0.05)),
                    "risk_score": float(pool_data.get("risk", 5.0)),
                }
            )

        return pools

    async def get_pool_details(self, pool_id: str) -> Dict[str, Any]:
        """
        Get detailed information about a specific pool

        Args:
            pool_id: Identifier for the pool

        Returns:
            Detailed pool information
        """
        try:
            if not self.use_mock_data:
                data = await self._api_request(f"pools/{pool_id}")
                if "error" not in data:
                    # Process API response
                    return self._process_pool_details_response(data)

            # Fall back to mock data
            pools = await self.data_provider.get_pools()
            for pool in pools:
                if pool["id"] == pool_id:
                    return pool

            raise Exception(f"Pool not found: {pool_id}")
        except Exception as e:
            self.logger.error(f"Error getting pool details: {e}")
            raise

    def _process_pool_details_response(self, data: Dict[str, Any]) -> Dict[str, Any]:
        """
        Process pool details API response

        Args:
            data: API response data

        Returns:
            Processed pool details
        """
        # Placeholder for actual API response processing
        return {
            "id": data.get("id", ""),
            "name": data.get("name", ""),
            "apy": float(data.get("apy", 0)),
            "tvl": float(data.get("tvl", 0)),
            "pair": data.get("tokens", []),
            "provider": "Helix",
            "volatility": float(data.get("volatility", 0.05)),
            "risk_score": float(data.get("risk", 5.0)),
            "volume_24h": float(data.get("volume_24h", 0)),
            "fee_tier": data.get("fee_tier", "0.3%"),
            "created_at": data.get("created_at", ""),
        }

    async def get_apy(self, pool_id: str) -> float:
        """
        Get the current APY for a specific pool

        Args:
            pool_id: Identifier for the pool

        Returns:
            Current APY as a percentage
        """
        pool_details = await self.get_pool_details(pool_id)
        return pool_details.get("apy", 0.0)

    async def get_tvl(self, pool_id: Optional[str] = None) -> Dict[str, float]:
        """
        Get total value locked for pools

        Args:
            pool_id: If provided, get TVL for specific pool, otherwise all pools

        Returns:
            Dictionary mapping pool IDs to TVL values
        """
        if pool_id:
            pool_details = await self.get_pool_details(pool_id)
            return {pool_id: pool_details.get("tvl", 0.0)}

        pools = await self.get_pools()
        return {pool["id"]: pool.get("tvl", 0.0) for pool in pools}

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
        try:
            if not self.use_mock_data:
                end_date = datetime.now().strftime("%Y-%m-%d")
                start_date = (datetime.now() - timedelta(days=days)).strftime(
                    "%Y-%m-%d"
                )
                data = await self._api_request(
                    f"pools/{pool_id}/history",
                    {"start_date": start_date, "end_date": end_date},
                )
                if "error" not in data:
                    # Process API response
                    return self._process_historical_data_response(data)

            # Fall back to mock data
            historical_data = await self.data_provider.get_historical_data(days)
            if pool_id in historical_data.get(self.protocol_id, {}):
                return historical_data[self.protocol_id][pool_id]

            raise Exception(f"Historical data not found for pool: {pool_id}")
        except Exception as e:
            self.logger.error(f"Error getting historical data: {e}")
            # Generate synthetic history if all else fails
            return self._generate_synthetic_history(pool_id, days)

    def _process_historical_data_response(
        self, data: Dict[str, Any]
    ) -> List[Dict[str, Any]]:
        """
        Process historical data API response

        Args:
            data: API response data

        Returns:
            Processed historical data
        """
        # Placeholder for actual API response processing
        history = []
        for item in data.get("history", []):
            history.append(
                {
                    "date": item.get("date", ""),
                    "apy": float(item.get("apy", 0)),
                    "tvl": float(item.get("tvl", 0)),
                    "volume_24h": float(item.get("volume_24h", 0)),
                }
            )

        return history

    def _generate_synthetic_history(
        self, pool_id: str, days: int
    ) -> List[Dict[str, Any]]:
        """
        Generate synthetic historical data when no real data is available

        Args:
            pool_id: Pool identifier
            days: Number of days

        Returns:
            List of synthetic data points
        """
        self.logger.warning(f"Generating synthetic historical data for {pool_id}")
        import random

        try:
            # Try to get current pool details as baseline
            pools = self.data_provider.pools
            pool = next((p for p in pools if p["id"] == pool_id), None)

            if not pool:
                base_apy = 10.0
                base_tvl = 1000000.0
            else:
                base_apy = pool.get("apy", 10.0)
                base_tvl = pool.get("tvl", 1000000.0)

            # Generate synthetic history
            history = []
            for i in range(days):
                date = datetime.now() - timedelta(days=i)
                # Add some random fluctuation
                apy = max(0.1, base_apy * (1 + (random.random() - 0.5) * 0.1))
                tvl = max(1000, base_tvl * (1 + (random.random() - 0.5) * 0.05))

                history.append(
                    {
                        "date": date.isoformat(),
                        "apy": round(apy, 2),
                        "tvl": round(tvl, 0),
                        "volume_24h": round(
                            tvl * random.uniform(0.05, 0.15), 0
                        ),  # 5-15% daily volume
                    }
                )

            return history

        except Exception as e:
            self.logger.error(f"Error generating synthetic history: {e}")
            # Return minimal synthetic data if everything fails
            return [
                {
                    "date": (datetime.now() - timedelta(days=i)).isoformat(),
                    "apy": 10.0,
                    "tvl": 1000000.0,
                    "volume_24h": 50000.0,
                }
                for i in range(days)
            ]

    async def get_assets(self, pool_id: str) -> List[Dict[str, Any]]:
        """
        Get assets in a specific pool

        Args:
            pool_id: Identifier for the pool

        Returns:
            List of assets in the pool with details
        """
        pool_details = await self.get_pool_details(pool_id)
        pair = pool_details.get("pair", [])

        assets = []
        for token in pair:
            assets.append(
                {
                    "symbol": token,
                    "name": self._get_token_name(token),
                    "address": self._get_token_address(token),
                    "decimals": 6 if token == "USDC" or token == "USDT" else 18,
                }
            )

        return assets

    def _get_token_name(self, symbol: str) -> str:
        """Get token name from symbol"""
        token_names = {
            "USDC": "USD Coin",
            "USDT": "Tether USD",
            "INJ": "Injective Protocol",
            "ATOM": "Cosmos",
            "DAI": "Dai Stablecoin",
        }
        return token_names.get(symbol, symbol)

    def _get_token_address(self, symbol: str) -> str:
        """Get token address from symbol"""
        # These would be actual Injective addresses in production
        token_addresses = {
            "USDC": "inj1...usdc",
            "USDT": "inj1...usdt",
            "INJ": "inj1...inj",
            "ATOM": "inj1...atom",
            "DAI": "inj1...dai",
        }
        return token_addresses.get(symbol, "inj1...unknown")

    async def get_protocol_info(self) -> Dict[str, Any]:
        """
        Get general information about the Helix protocol

        Returns:
            Protocol information including name, description, etc.
        """
        try:
            if not self.use_mock_data:
                data = await self._api_request("info")
                if "error" not in data:
                    return data

            # Fall back to config/mock data
            return {
                "name": "Helix",
                "description": "Helix is a DeFi protocol on Injective Chain",
                "website": "https://helix.injective.network",
                "tvl": sum((await self.get_tvl()).values()),
                "pool_count": len(await self.get_pools()),
            }
        except Exception as e:
            self.logger.error(f"Error getting protocol info: {e}")
            return {
                "name": "Helix",
                "description": "Helix is a DeFi protocol on Injective Chain",
                "error": str(e),
            }

    async def estimate_protocol_risk(self) -> float:
        """
        Estimate overall Helix protocol risk on a scale of 1-10

        Returns:
            Risk score (1 = lowest risk, 10 = highest risk)
        """
        # Helix-specific risk factors
        try:
            # Get TVL as a factor
            tvl_data = await self.get_tvl()
            total_tvl = sum(tvl_data.values())

            # Larger TVL = lower base risk
            tvl_factor = max(1.0, 10.0 - min(9.0, total_tvl / 1_000_000 * 0.5))

            # Helix is considered a medium-risk protocol (4-6 range)
            protocol_base_risk = 5.0

            # Calculate final risk score
            risk_score = (tvl_factor * 0.6) + (protocol_base_risk * 0.4)

            return round(risk_score, 1)
        except Exception as e:
            self.logger.warning(f"Error estimating protocol risk: {e}")
            return 5.0  # Default moderate risk
