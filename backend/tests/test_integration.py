# tests/test_integration.py
import asyncio
import sys
import os
import logging
from datetime import datetime

# Add the parent directory to the path to import modules
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..")))

# Import adapter and agent classes
from src.adapters.helix_adapter import HelixAdapter
from src.adapters.hydro_adapter import HydroAdapter
from src.adapters.neptune_adapter import NeptuneAdapter
from src.agent.helix_agent import HelixAgent
from src.agent.hydro_agent import HydroAgent
from src.agent.neptune_agent import NeptuneAgent

# Configure logging
logging.basicConfig(
    level=logging.INFO, format="%(asctime)s - %(name)s - %(levelname)s - %(message)s"
)
logger = logging.getLogger("astrobalance.test")


async def test_agent_adapter_integration(adapter_class, agent_class, protocol_name):
    """Test integration between an agent and its adapter"""
    print(f"\n===== Testing {protocol_name} Agent-Adapter Integration =====")

    try:
        # Create adapter instance
        adapter = adapter_class()

        # Create agent with the adapter
        agent = agent_class(adapter)

        # Test complete analysis flow
        print(f"Testing complete analysis flow...")

        # 1. Adapter gets pools
        pools = await adapter.get_pools()
        print(f"Adapter found {len(pools)} pools")

        # 2. Agent analyzes opportunities
        analysis = await agent.analyze_opportunities()
        print(f"Agent analyzed {len(analysis.get('opportunities', []))} opportunities")

        # 3. Agent performs risk assessment
        risk = await agent.get_risk_assessment()
        print(f"Agent assessed overall risk: {risk.get('overall_risk', 0)}/10")

        # 4. Agent provides recommendations
        recommendations = await agent.get_recommended_pools("moderate")
        print(f"Agent provided {len(recommendations)} recommendations")

        # 5. Check that the analysis uses data from the adapter
        if pools and analysis.get("opportunities", []):
            # Verify that pool IDs from adapter appear in agent analysis
            adapter_pool_ids = {p["id"] for p in pools}
            agent_pool_ids = {p["id"] for p in analysis.get("opportunities", [])}

            matching_ids = adapter_pool_ids.intersection(agent_pool_ids)

            print(
                f"Data consistency check: {len(matching_ids)}/{len(adapter_pool_ids)} pool IDs match"
            )

            if len(matching_ids) == len(adapter_pool_ids):
                print("✅ All adapter data was correctly processed by the agent")
            else:
                print("⚠️ Some adapter data may not have been processed by the agent")

        print(
            f"\n✅ {protocol_name} Agent-Adapter Integration tests completed successfully"
        )
        return True

    except Exception as e:
        print(f"\n❌ Error testing {protocol_name} Agent-Adapter Integration: {e}")
        import traceback

        traceback.print_exc()
        return False


async def main():
    """Run integration tests for all protocols"""
    # Test each protocol's agent-adapter integration
    helix_result = await test_agent_adapter_integration(
        HelixAdapter, HelixAgent, "Helix"
    )

    hydro_result = await test_agent_adapter_integration(
        HydroAdapter, HydroAgent, "Hydro"
    )

    neptune_result = await test_agent_adapter_integration(
        NeptuneAdapter, NeptuneAgent, "Neptune"
    )

    # Summary
    print("\n===== Integration Test Summary =====")
    print(f"Helix Integration: {'✅ PASS' if helix_result else '❌ FAIL'}")
    print(f"Hydro Integration: {'✅ PASS' if hydro_result else '❌ FAIL'}")
    print(f"Neptune Integration: {'✅ PASS' if neptune_result else '❌ FAIL'}")


if __name__ == "__main__":
    asyncio.run(main())
