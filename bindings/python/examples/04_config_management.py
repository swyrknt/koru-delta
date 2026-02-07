"""
Config Management Example

Demonstrates KoruDelta for versioned configuration:
- Feature flags with history
- A/B test configuration
- Rollback capabilities
- Environment-specific settings
"""

import asyncio
from koru_delta import Database, Config


async def main():
    """Demonstrate config management capabilities."""
    async with Database() as db:
        print("=" * 60)
        print("Config Management Demo")
        print("=" * 60)
        
        # --- FEATURE FLAGS ---
        print("\n--- Feature Flags ---")
        
        # Initial feature flag state
        await db.put("features", "dark-mode", {
            "enabled": False,
            "rollout_percentage": 0,
            "target_users": [],
            "description": "Dark mode UI theme",
            "owner": "frontend-team"
        })
        print("✓ Created: dark-mode feature (disabled)")
        
        # Enable for internal testing
        await db.put("features", "dark-mode", {
            "enabled": True,
            "rollout_percentage": 10,
            "target_users": ["employee-1", "employee-2"],
            "description": "Dark mode UI theme",
            "owner": "frontend-team"
        })
        print("✓ Updated: dark-mode enabled for 10% (internal testing)")
        
        # Gradual rollout
        await db.put("features", "dark-mode", {
            "enabled": True,
            "rollout_percentage": 50,
            "target_users": [],
            "description": "Dark mode UI theme",
            "owner": "frontend-team"
        })
        print("✓ Updated: dark-mode at 50% rollout")
        
        # Full rollout
        await db.put("features", "dark-mode", {
            "enabled": True,
            "rollout_percentage": 100,
            "target_users": [],
            "description": "Dark mode UI theme",
            "owner": "frontend-team"
        })
        print("✓ Updated: dark-mode at 100% rollout")
        
        # --- API RATE LIMITS ---
        print("\n--- API Rate Limits ---")
        
        rate_limits = {
            "free-tier": {"requests_per_minute": 60, "requests_per_hour": 1000},
            "pro-tier": {"requests_per_minute": 600, "requests_per_hour": 10000},
            "enterprise": {"requests_per_minute": 6000, "requests_per_hour": 100000}
        }
        
        for tier, limits in rate_limits.items():
            await db.put("rate-limits", tier, {
                **limits,
                "updated_at": "2026-02-06T10:00:00Z",
                "updated_by": "platform-team"
            })
            print(f"✓ Set rate limits for {tier}: {limits['requests_per_minute']}/min")
        
        # --- ENVIRONMENT CONFIG ---
        print("\n--- Environment Configuration ---")
        
        environments = {
            "development": {
                "api_endpoint": "http://localhost:8080",
                "debug": True,
                "log_level": "debug",
                "cache_ttl": 60
            },
            "staging": {
                "api_endpoint": "https://api-staging.example.com",
                "debug": False,
                "log_level": "info",
                "cache_ttl": 300
            },
            "production": {
                "api_endpoint": "https://api.example.com",
                "debug": False,
                "log_level": "warn",
                "cache_ttl": 3600
            }
        }
        
        for env, config in environments.items():
            await db.put("environments", env, config)
            print(f"✓ Configured {env} environment")
        
        # --- ROLLBACK DEMONSTRATION ---
        print("\n--- Rollback Demonstration ---")
        
        # Show history of dark-mode feature
        print("\nDark-mode rollout history:")
        history = await db.history("features", "dark-mode")
        for i, entry in enumerate(history, 1):
            value = entry.get("value", {})
            pct = value.get("rollout_percentage", 0)
            enabled = value.get("enabled", False)
            status = "enabled" if enabled else "disabled"
            print(f"  {i}. {status} @ {pct}% rollout")
        
        # Simulate incident: need to rollback dark-mode
        print("\n⚠️  Simulating incident: dark-mode causing UI glitches")
        print("   Performing rollback to previous version...")
        
        # Get previous version (before 100%)
        previous_version = history[-2]["value"]  # 50% rollout version
        await db.put("features", "dark-mode", {
            **previous_version,
            "rollback_reason": "UI glitches reported",
            "rolled_back_by": "oncall-engineer",
            "rolled_back_at": "2026-02-06T11:30:00Z"
        })
        
        current = await db.get("features", "dark-mode")
        print(f"\n✓ Rollback complete: dark-mode now at {current['rollout_percentage']}%")
        
        # --- CONFIG VALIDATION ---
        print("\n--- Config Validation ---")
        
        # Verify all required configs exist
        required_features = ["dark-mode"]
        required_tiers = ["free-tier", "pro-tier", "enterprise"]
        required_envs = ["development", "staging", "production"]
        
        print("\nValidating configuration completeness:")
        
        all_features = await db.list_keys("features")
        for feature in required_features:
            status = "✓" if feature in all_features else "✗"
            print(f"  {status} Feature: {feature}")
        
        all_tiers = await db.list_keys("rate-limits")
        for tier in required_tiers:
            status = "✓" if tier in all_tiers else "✗"
            print(f"  {status} Rate limit tier: {tier}")
        
        all_envs = await db.list_keys("environments")
        for env in required_envs:
            status = "✓" if env in all_envs else "✗"
            print(f"  {status} Environment: {env}")
        
        # --- CONFIG COMPARISON ---
        print("\n--- Environment Comparison ---")
        
        print("\nComparing dev vs prod configurations:")
        dev_config = await db.get("environments", "development")
        prod_config = await db.get("environments", "production")
        
        keys = set(dev_config.keys()) | set(prod_config.keys())
        for key in sorted(keys):
            dev_val = dev_config.get(key, "N/A")
            prod_val = prod_config.get(key, "N/A")
            match = "✓" if dev_val == prod_val else "≠"
            print(f"  {match} {key}: dev={dev_val}, prod={prod_val}")
        
        # --- FINAL STATE ---
        print("\n--- Final Configuration State ---")
        
        stats = await db.stats()
        print(f"\nDatabase stats:")
        print(f"  Total keys: {stats['key_count']}")
        print(f"  Namespaces: {stats['namespace_count']}")
        
        namespaces = ["features", "rate-limits", "environments"]
        for ns in namespaces:
            keys = await db.list_keys(ns)
            print(f"  {ns}: {len(keys)} config items")
        
        print("\n" + "=" * 60)
        print("✓ Config Management Demo Complete!")
        print("=" * 60)
        print("\nKey Features Demonstrated:")
        print("  • Versioned feature flags with rollout tracking")
        print("  • Environment-specific configuration")
        print("  • One-click rollback to any previous version")
        print("  • Complete audit trail of all changes")
        print("  • Config validation and comparison")


if __name__ == "__main__":
    asyncio.run(main())
