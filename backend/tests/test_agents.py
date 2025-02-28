# tests/test_agents.py
import asyncio
import sys
import os
import logging
from datetime import datetime

# Add the parent directory to the path to import modules
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..")))

# Import agent classes
from src.agent.helix_agent import HelixAgent
from src.agent.hydro_agent import HydroAgent
from src.agent.neptune_agent import NeptuneAgent

# Configure logging
logging.basicConfig(
    level=logging.INFO, format="%(asctime)s - %(name)s - %(levelname)s - %(message)s"
)
logger = logging.getLogger("astrobalance.test")


async def test_agent(agent, agent_name):
    """Test a protocol agent's basic functionality"""
    print(f"\n===== Testing {agent_name} Agent =====")

    try:
        # Test analyzing opportunities
        print(f"\nTesting analyze_opportunities()...")
        analysis = await agent.analyze_opportunities()
        opportunities = analysis.get("opportunities", [])
        print(f"Found {len(opportunities)} opportunities")

        if "top_opportunity" in analysis and analysis["top_opportunity"]:
            top = analysis["top_opportunity"]
            print(f"Top opportunity: {top.get('name', '')}")
            print(f"  APY: {top.get('apy', 0)}%")
            print(f"  Risk-adjusted return: {top.get('risk_adjusted_return', 0)}")

        # Test risk assessment
        print(f"\nTesting get_risk_assessment()...")
        risk = await agent.get_risk_assessment()
        print(f"Overall risk score: {risk.get('overall_risk', 0)}/10")
        print(f"Protocol risk: {risk.get('protocol_risk', 0)}/10")

        # Test recommendations for different risk profiles
        for profile in ["conservative", "moderate", "aggressive"]:
            print(f"\nTesting get_recommended_pools() for {profile} profile...")
            recommendations = await agent.get_recommended_pools(profile)
            print(f"Found {len(recommendations)} recommendations")

            if recommendations:
                rec = recommendations[0]
                print(f"Top recommendation: {rec.get('name', '')}")
                print(f"  APY: {rec.get('apy', 0)}%")
                print(f"  Note: {rec.get('recommendation_note', '')}")

        # Test correlation estimation
        other_protocols = ["helix", "hydro", "neptune"]
        other_protocols.remove(agent_name.lower())

        for other in other_protocols:
            print(f"\nTesting estimate_correlation() with {other}...")
            correlation = await agent.estimate_correlation(other)
            print(f"Correlation with {other}: {correlation}")

        # Test historical performance
        print(f"\nTesting get_historical_performance()...")
        history = await agent.get_historical_performance(days=7)
        if "error" not in history:
            print(
                f"Historical data retrieved for {len(history.get('pools', {}))} pools"
            )
            metrics = history.get("protocol_metrics", [])
            if metrics:
                print(f"Protocol metrics available for {len(metrics)} days")

        # Test agent-specific methods
        if agent_name.lower() == "hydro":
            print(f"\nTesting Hydro-specific methods...")
            efficiency = await agent.analyze_stable_pool_efficiency()
            print(
                f"Stable pool analysis: {efficiency.get('average_efficiency_score', 0)} average score"
            )

        elif agent_name.lower() == "neptune":
            print(f"\nTesting Neptune-specific methods...")
            strategy_analysis = await agent.analyze_strategy_efficiency()
            print(
                f"Strategy efficiency: {strategy_analysis.get('average_efficiency_score', 0)} average score"
            )

        print(f"\n✅ {agent_name} Agent tests completed successfully")
        return True

    except Exception as e:
        print(f"\n❌ Error testing {agent_name} Agent: {e}")
        import traceback

        traceback.print_exc()
        return False


async def main():
    """Run tests for all agents"""
    # Create agent instances
    helix_agent = HelixAgent()
    hydro_agent = HydroAgent()
    neptune_agent = NeptuneAgent()

    # Test each agent
    helix_result = await test_agent(helix_agent, "Helix")
    hydro_result = await test_agent(hydro_agent, "Hydro")
    neptune_result = await test_agent(neptune_agent, "Neptune")

    # Summary
    print("\n===== Test Summary =====")
    print(f"Helix Agent: {'✅ PASS' if helix_result else '❌ FAIL'}")
    print(f"Hydro Agent: {'✅ PASS' if hydro_result else '❌ FAIL'}")
    print(f"Neptune Agent: {'✅ PASS' if neptune_result else '❌ FAIL'}")


if __name__ == "__main__":
    asyncio.run(main())
