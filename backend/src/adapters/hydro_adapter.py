"""
Hydro Protocol Adapter

This module implements the adapter for the Hydro protocol on Injective.
It handles all communication with the Hydro protocol endpoints.
"""

import yaml
import aiohttp
import os
import json
import logging
import random
from typing import Dict, List, Any, Optional
from datetime import datetime, timedelta

from .base_adapter import BaseAdapter
from src.data.market_data import HydroData


class HydroAdapter(BaseAdapter):
    """Adapter for Hydro protocol"""

    def __init__(self, config_path: Optional[str] = None):
        """
        Initialize Hydro adapter

        Args:
            config_path: Path to configuration file (optional)
        """
        super().__init__("hydro", config_path)
        self.config = self._load_config(config_path)
        self.api_url = self.config.get("api_url", "https://hydro.injective.network/api")

        # Initialize the data provider, with fallback to mock data
        self.data_provider = HydroData(config_path)
        self.use_mock_data = self.config.get("enable_mock_data", True)

        # Token metadata for Hydro
        self.token_metadata = {
            "USDC": {
                "name": "USD Coin",
                "address": "inj1k9x9k779hwz8r9f5mcjs9qnkef9n4l9z6vv7f5",
                "decimals": 6,
            },
            "USDT": {
                "name": "Tether USD",
                "address": "inj1ktmz4uzdgn7ydvlnfgmkssh0jn9mje3cn3czug",
                "decimals": 6,
            },
            "INJ": {"name": "Injective Protocol", "address": "inj", "decimals": 18},
            "ATOM": {
                "name": "Cosmos",
                "address": "inj1cdwt8g7nxgtg6gqv0lnzdhnckcfqzh8v058frc",
                "decimals": 6,
            },
            "DAI": {
                "name": "Dai Stablecoin",
                "address": "inj1lxkgvxrf8q0ym3qygsahm74f09keq5as2psk7s",
                "decimals": 18,
            },
        }

    def _load_config(self, config_path: Optional[str] = None) -> Dict[str, Any]:
        """
        Load protocol configuration

        Args:
            config_path: Path to configuration file

        Returns:
            Configuration dictionary
        """
        default_path = os.path.join("config", "hydro.yaml")
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
        Make an API request to the Hydro protocol

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
        Get all liquidity pools from Hydro

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
        # This would be implemented based on actual Hydro API response format
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
                    "provider": "Hydro",
                    "volatility": float(pool_data.get("volatility", 0.05)),
                    "risk_score": float(pool_data.get("risk", 5.0)),
                    "pool_type": pool_data.get("pool_type", "Stable"),
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
            "provider": "Hydro",
            "volatility": float(data.get("volatility", 0.05)),
            "risk_score": float(data.get("risk", 5.0)),
            "volume_24h": float(data.get("volume_24h", 0)),
            "pool_type": data.get("pool_type", "Stable"),
            "swap_fee": data.get("swap_fee", "0.04%"),
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

        try:
            # Try to get current pool details as baseline
            pools = self.data_provider.pools
            pool = next((p for p in pools if p["id"] == pool_id), None)

            if not pool:
                # Hydro pools tend to have lower APYs for stablecoin pools
                if "usdc_usdt" in pool_id.lower() or "usdc_dai" in pool_id.lower():
                    base_apy = 8.0
                    volatility_factor = 0.03  # Lower volatility for stable pairs
                else:
                    base_apy = 25.0
                    volatility_factor = 0.06  # Higher volatility for other pairs

                base_tvl = 3000000.0  # Hydro tends to have higher TVL
            else:
                base_apy = pool.get("apy", 8.0)
                base_tvl = pool.get("tvl", 3000000.0)
                # Check if it's a stable pool
                if "USDT" in pool.get("pair", []) and "USDC" in pool.get("pair", []):
                    volatility_factor = 0.03
                else:
                    volatility_factor = 0.06

            # Generate synthetic history
            history = []
            for i in range(days):
                date = datetime.now() - timedelta(days=i)
                # Add some random fluctuation - adjusted by volatility factor
                apy = max(
                    0.1,
                    base_apy * (1 + (random.random() - 0.5) * volatility_factor * 2),
                )
                tvl = max(1000, base_tvl * (1 + (random.random() - 0.5) * 0.04))

                history.append(
                    {
                        "date": date.isoformat(),
                        "apy": round(apy, 2),
                        "tvl": round(tvl, 0),
                        "volume_24h": round(
                            tvl * random.uniform(0.02, 0.12), 0
                        ),  # 2-12% daily volume
                    }
                )

            return history

        except Exception as e:
            self.logger.error(f"Error generating synthetic history: {e}")
            # Return minimal synthetic data if everything fails
            return [
                {
                    "date": (datetime.now() - timedelta(days=i)).isoformat(),
                    "apy": 8.0,
                    "tvl": 3000000.0,
                    "volume_24h": 120000.0,
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
            token_info = self.token_metadata.get(
                token,
                {"name": token, "address": f"inj1...{token.lower()}", "decimals": 18},
            )

            assets.append(
                {
                    "symbol": token,
                    "name": token_info["name"],
                    "address": token_info["address"],
                    "decimals": token_info["decimals"],
                }
            )

        return assets

    async def get_protocol_info(self) -> Dict[str, Any]:
        """
        Get general information about the Hydro protocol

        Returns:
            Protocol information including name, description, etc.
        """
        try:
            if not self.use_mock_data:
                data = await self._api_request("info")
                if "error" not in data:
                    return data

            # Fall back to config/mock data
            tvl_data = await self.get_tvl()
            pool_data = await self.get_pools()

            return {
                "name": "Hydro Protocol",
                "description": "Hydro is a concentrated liquidity AMM protocol on Injective Chain",
                "website": "https://hydro.injective.network",
                "tvl": sum(tvl_data.values()),
                "pool_count": len(pool_data),
                "features": ["Concentrated Liquidity", "Stable Swaps", "Dynamic Fees"],
                "audit_status": "Audited",
                "launch_date": "2022-11-15",
                "specialization": "Stablecoin liquidity",
            }
        except Exception as e:
            self.logger.error(f"Error getting protocol info: {e}")
            return {
                "name": "Hydro Protocol",
                "description": "Hydro is a concentrated liquidity AMM protocol on Injective Chain",
                "error": str(e),
            }

    async def estimate_protocol_risk(self) -> float:
        """
        Estimate overall Hydro protocol risk on a scale of 1-10

        Returns:
            Risk score (1 = lowest risk, 10 = highest risk)
        """
        # Hydro-specific risk factors
        try:
            # Get TVL as a factor
            tvl_data = await self.get_tvl()
            total_tvl = sum(tvl_data.values())

            # Larger TVL = lower base risk
            tvl_factor = max(1.0, 10.0 - min(9.0, total_tvl / 1_000_000 * 0.5))

            # Hydro is considered a lower-risk protocol (3-5 range) due to focus on stablecoins
            protocol_base_risk = 4.0

            # Hydro-specific risk considerations
            protocol_age = 1.5  # Years since launch
            age_factor = max(1.0, 7.0 - protocol_age)  # Newer protocols are riskier

            audit_factor = 3.0  # Audited protocols are less risky

            # Calculate stable pool ratio - Hydro specializes in stable pools
            pools = await self.get_pools()
            stable_pools = sum(1 for p in pools if p.get("pool_type", "") == "Stable")
            stable_ratio = stable_pools / len(pools) if pools else 0.5

            # More stable pools = lower risk
            stable_factor = 6.0 - (stable_ratio * 3.0)

            # Calculate final risk score with weights
            weights = {
                "tvl": 0.35,
                "base": 0.25,
                "age": 0.15,
                "audit": 0.1,
                "stable": 0.15,
            }

            risk_score = (
                tvl_factor * weights["tvl"]
                + protocol_base_risk * weights["base"]
                + age_factor * weights["age"]
                + audit_factor * weights["audit"]
                + stable_factor * weights["stable"]
            )

            return round(risk_score, 1)
        except Exception as e:
            self.logger.warning(f"Error estimating protocol risk: {e}")
            return 4.0  # Default slightly lower than moderate risk

    async def get_pool_parameters(self, pool_id: str) -> Dict[str, Any]:
        """
        Get detailed parameters for a Hydro pool

        Args:
            pool_id: Pool identifier

        Returns:
            Pool parameters
        """
        try:
            # Hydro has specific pool types and parameters
            if not self.use_mock_data:
                data = await self._api_request(f"pools/{pool_id}/parameters")
                if "error" not in data:
                    return data

            # Fall back to mock data
            pool = await self.get_pool_details(pool_id)

            # Generate mock parameters based on pool type
            is_stable = "USDT" in pool_id and "USDC" in pool_id

            if is_stable:
                return {
                    "pool_id": pool_id,
                    "pool_type": "Stable",
                    "amp": 100,  # Amplification parameter for stable pools
                    "swap_fee": "0.04%",
                    "admin_fee": "10%",
                    "price_range": "0.98 - 1.02",
                    "min_trade_size": "10 USDC",
                    "oracle_enabled": True,
                    "created_at": "2022-12-01T00:00:00Z",
                }
            else:
                return {
                    "pool_id": pool_id,
                    "pool_type": "Weighted",
                    "weights": {token: "50%" for token in pool.get("pair", [])},
                    "swap_fee": "0.3%",
                    "admin_fee": "10%",
                    "price_range": "Full Range",
                    "min_trade_size": "1 USDC",
                    "oracle_enabled": False,
                    "created_at": "2023-01-15T00:00:00Z",
                }

        except Exception as e:
            self.logger.error(f"Error getting pool parameters: {e}")
            return {"pool_id": pool_id, "error": str(e)}

    async def get_fee_structure(self, pool_id: Optional[str] = None) -> Dict[str, Any]:
        """
        Get fee structure for Hydro pools

        Args:
            pool_id: Optional pool to get specific fee info

        Returns:
            Fee structure information
        """
        try:
            if pool_id:
                # Hydro has different fee tiers based on pool type
                pool = await self.get_pool_details(pool_id)
                pool_type = pool.get("pool_type", "Weighted")

                if pool_type == "Stable":
                    swap_fee = "0.04%"
                    admin_fee = "10%"
                else:
                    swap_fee = "0.3%"
                    admin_fee = "10%"

                return {
                    "pool_id": pool_id,
                    "pool_type": pool_type,
                    "swap_fee": swap_fee,
                    "admin_fee": admin_fee,
                    "fee_recipient": "Hydro DAO Treasury",
                }
            else:
                # Return general fee structure
                return {
                    "fee_tiers": [
                        {
                            "tier": "0.01%",
                            "description": "Ultra-low fee for highly liquid stable pairs",
                        },
                        {"tier": "0.04%", "description": "Low fee for stable pairs"},
                        {
                            "tier": "0.3%",
                            "description": "Standard fee for weighted pools",
                        },
                        {"tier": "1%", "description": "High fee for exotic pairs"},
                    ],
                    "admin_fee": "10% of swap fees",
                    "fee_distribution": {
                        "protocol_treasury": "50%",
                        "stakers": "30%",
                        "safety_fund": "20%",
                    },
                }

        except Exception as e:
            self.logger.error(f"Error getting fee structure: {e}")
            return {"error": str(e)}
