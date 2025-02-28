import asyncio
import sys
import os
import logging
from datetime import datetime

# Add the parent directory to the path to import modules
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..")))

# Import adapter classes
from src.adapters.helix_adapter import HelixAdapter
from src.adapters.hydro_adapter import HydroAdapter
from src.adapters.neptune_adapter import NeptuneAdapter

# Configure logging
logging.basicConfig(
    level=logging.INFO, format="%(asctime)s - %(name)s - %(levelname)s - %(message)s"
)
logger = logging.getLogger("astrobalance.test")


async def test_adapter(adapter, adapter_name):
    """Test a protocol adapter's basic functionality"""
    print(f"\n===== Testing {adapter_name} Adapter =====")

    try:
        # Test getting pools
        print(f"\nTesting get_pools()...")
        pools = await adapter.get_pools()
        print(f"Found {len(pools)} pools")

        if len(pools) > 0:
            # Show the first pool
            print(f"First pool: {pools[0]['name']} (ID: {pools[0]['id']})")
            print(f"  APY: {pools[0]['apy']}%")
            print(f"  TVL: ${pools[0]['tvl']:,.2f}")

            # Test getting pool details
            pool_id = pools[0]["id"]
            print(f"\nTesting get_pool_details() for {pool_id}...")
            pool_details = await adapter.get_pool_details(pool_id)
            print(f"Pool details: {pool_details['name']}")

            # Test getting historical data
            print(f"\nTesting get_historical_data() for {pool_id}...")
            history = await adapter.get_historical_data(pool_id, days=7)
            print(f"Got {len(history)} days of history")
            if len(history) > 0:
                print(
                    f"Most recent day: {history[0]['date']}, APY: {history[0]['apy']}%"
                )

            # Test getting assets
            print(f"\nTesting get_assets() for {pool_id}...")
            assets = await adapter.get_assets(pool_id)
            print(f"Found {len(assets)} assets in pool")
            for asset in assets:
                print(f"  {asset['symbol']} ({asset['name']})")

        # Test protocol info
        print(f"\nTesting get_protocol_info()...")
        info = await adapter.get_protocol_info()
        print(f"Protocol: {info['name']}")
        print(f"TVL: ${info.get('tvl', 0):,.2f}")

        # Test protocol risk
        print(f"\nTesting estimate_protocol_risk()...")
        risk = await adapter.estimate_protocol_risk()
        print(f"Protocol risk score: {risk}/10")

        # Test additional adapter-specific methods (if available)
        if adapter_name.lower() == "hydro":
            print(f"\nTesting Hydro-specific methods...")
            pool_id = pools[0]["id"] if pools else "hydro_usdc_usdt"
            params = await adapter.get_pool_parameters(pool_id)
            print(f"Pool parameters: {params.get('pool_type', '')} type")

        elif adapter_name.lower() == "neptune":
            print(f"\nTesting Neptune-specific methods...")
            pool_id = pools[0]["id"] if pools else "neptune_usdc_atom"
            strategy = await adapter.get_strategy_details(pool_id)
            print(f"Strategy: {strategy.get('strategy_type', '')}")
            print(f"Performance fee: {strategy.get('performance_fee', '')}")

        print(f"\n✅ {adapter_name} Adapter tests completed successfully")
        return True

    except Exception as e:
        print(f"\n❌ Error testing {adapter_name} Adapter: {e}")
        import traceback

        traceback.print_exc()
        return False


async def main():
    """Run tests for all adapters"""
    # Create adapter instances
    helix_adapter = HelixAdapter()
    hydro_adapter = HydroAdapter()
    neptune_adapter = NeptuneAdapter()

    # Test each adapter
    helix_result = await test_adapter(helix_adapter, "Helix")
    hydro_result = await test_adapter(hydro_adapter, "Hydro")
    neptune_result = await test_adapter(neptune_adapter, "Neptune")

    # Summary
    print("\n===== Test Summary =====")
    print(f"Helix Adapter: {'✅ PASS' if helix_result else '❌ FAIL'}")
    print(f"Hydro Adapter: {'✅ PASS' if hydro_result else '❌ FAIL'}")
    print(f"Neptune Adapter: {'✅ PASS' if neptune_result else '❌ FAIL'}")


if __name__ == "__main__":
    asyncio.run(main())
