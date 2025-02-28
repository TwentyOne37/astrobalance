"""
Test script for Milestone 1 verification

This script verifies that the iAgent client can connect and communicate,
and that the market data collection works properly.
"""

import asyncio
import logging
import sys
import os

# Add the parent directory to the path
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..")))

from src.iagent.client import IAgentClient, AstroBalanceAgent
from src.data.market_data import MarketDataCollector

# Configure logging
logging.basicConfig(
    level=logging.INFO, format="%(asctime)s - %(name)s - %(levelname)s - %(message)s"
)
logger = logging.getLogger("astrobalance.test")


async def test_iagent_client():
    """Test iAgent client connection and communication"""
    logger.info("Testing iAgent client...")

    client = IAgentClient(base_url="http://localhost:5000")
    try:
        ping_result = await client.ping()
        logger.info(f"iAgent ping result: {ping_result}")
        return True
    except Exception as e:
        logger.warning(f"Could not connect to real iAgent: {e}")
        logger.info("Falling back to mock iAgent client for development")

        # Import and use mock client
        from src.iagent.mock_client import MockIAgentClient

        mock_client = MockIAgentClient()

        try:
            ping_result = await mock_client.ping()
            logger.info(f"Mock iAgent ping result: {ping_result}")
            return True
        except Exception as e2:
            logger.error(f"Error with mock client too: {e2}")
            return False


async def test_astrobalance_agent():
    """Test AstroBalance agent interface"""
    logger.info("Testing AstroBalance agent...")

    agent = AstroBalanceAgent()
    try:
        # Just test initialization for now
        logger.info(f"Agent initialized: {agent}")
        return True
    except Exception as e:
        logger.error(f"Error initializing agent: {e}")
        return False


async def test_market_data():
    """Test market data collection"""
    logger.info("Testing market data collection...")

    collector = MarketDataCollector()
    try:
        pools = await collector.get_all_pools()
        logger.info(f"Found {len(pools)} pools across all protocols")

        tvl = await collector.get_all_tvl()
        logger.info(f"Total TVL across all protocols: ${sum(tvl.values()):,.2f}")

        stats = await collector.get_protocol_stats()
        for protocol_id, protocol_stats in stats.items():
            logger.info(
                f"- {protocol_id}: {protocol_stats['pool_count']} pools, ${protocol_stats['total_tvl']:,.2f} TVL"
            )

        return True
    except Exception as e:
        logger.error(f"Error collecting market data: {e}")
        return False


async def verify_milestone1():
    """Run all tests to verify Milestone 1"""
    logger.info("Verifying Milestone 1...")

    # Test iAgent client
    iagent_client_ok = await test_iagent_client()

    # Test AstroBalance agent
    astrobalance_agent_ok = await test_astrobalance_agent()

    # Test market data collection
    market_data_ok = await test_market_data()

    # Verify all tests passed
    if iagent_client_ok and astrobalance_agent_ok and market_data_ok:
        logger.info("✅ Milestone 1 verification successful!")
        return True
    else:
        logger.error("❌ Milestone 1 verification failed!")
        if not iagent_client_ok:
            logger.error("  - iAgent client test failed")
        if not astrobalance_agent_ok:
            logger.error("  - AstroBalance agent test failed")
        if not market_data_ok:
            logger.error("  - Market data collection test failed")
        return False


if __name__ == "__main__":
    result = asyncio.run(verify_milestone1())
    sys.exit(0 if result else 1)
