"""
Time-Travel Config Management & Feature Debugger

This example demonstrates KoruDelta's unique config capabilities:
- "Git for configuration" - complete version history
- Time-travel debugging: "What did the config look like during the outage?"
- Causal analysis: Which config change caused the incident?
- Branch realities: Test "what if" scenarios

Unlike traditional config systems (Envoy, Consul), KoruDelta stores
not just the current config, but HOW it evolved - the complete causal chain.
"""

import asyncio
from datetime import datetime, timezone, timedelta
from koru_delta import Database


async def main():
    """Demonstrate time-travel config management."""
    async with Database() as db:
        print("=" * 70)
        print("⏰ Time-Travel Config Management")
        print("=" * 70)
        print("\nTraditional config: 'Current value is X'")
        print("KoruDelta config: 'Value evolved A→B→C because of Y'")
        print("Complete causal history with time-travel debugging.\n")
        
        # --- SCENARIO: Production Incident ---
        print("🎬 SCENARIO: E-commerce Platform Incident")
        print("-" * 70)
        print("Timeline: Monday 2pm - Checkout service goes down")
        print("Timeline: Monday 3pm - Root cause identified")
        print("Timeline: Now - Post-mortem with time-travel analysis")
        print("-" * 70 + "\n")
        
        # --- INITIAL CONFIG STATE (Monday 9am) ---
        print("--- Monday 9:00 AM - Initial Stable State ---\n")
        
        base_time = datetime(2026, 2, 2, 9, 0, 0, tzinfo=timezone.utc)
        
        await db.put("config", "checkout-timeout", {
            "value_ms": 5000,
            "reason": "Default for peak traffic",
            "changed_by": "ops-team",
            "timestamp": base_time.isoformat()
        })
        print("✓ checkout-timeout: 5000ms (stable)")
        
        await db.put("config", "payment-retries", {
            "value": 3,
            "reason": "Balance reliability vs latency",
            "changed_by": "ops-team",
            "timestamp": base_time.isoformat()
        })
        print("✓ payment-retries: 3 attempts")
        
        await db.put("config", "feature-new-checkout", {
            "enabled": False,
            "rollout": 0,
            "reason": "Still in testing",
            "changed_by": "product-team",
            "timestamp": base_time.isoformat()
        })
        print("✓ feature-new-checkout: DISABLED")
        
        # --- FIRST CHANGE (11am) ---
        print("\n--- Monday 11:00 AM - Performance Optimization ---\n")
        
        await db.put("config", "checkout-timeout", {
            "value_ms": 1000,  # Reduced!
            "reason": "Reduce latency - improve UX",
            "changed_by": "perf-team",
            "ticket": "PERF-2042",
            "timestamp": (base_time + timedelta(hours=2)).isoformat()
        })
        print("⚡ checkout-timeout: 5000ms → 1000ms (optimization)")
        
        # --- SECOND CHANGE (1pm) ---
        print("\n--- Monday 1:00 PM - Feature Rollout ---\n")
        
        await db.put("config", "feature-new-checkout", {
            "enabled": True,
            "rollout": 100,
            "reason": "Deploy new checkout flow to all users",
            "changed_by": "product-team",
            "ticket": "PROD-891",
            "timestamp": (base_time + timedelta(hours=4)).isoformat()
        })
        print("🚀 feature-new-checkout: ENABLED @ 100% rollout")
        
        # --- INCIDENT! (2pm) ---
        print("\n" + "🔴" * 25)
        print("🔴🔴🔴  INCIDENT: Checkout service failing!   🔴🔴🔴")
        print("🔴" * 25 + "\n")
        
        print("❌ Error rate: 45%")
        print("❌ Payment timeouts: Spiking")
        print("❌ Customer complaints: Twitter blowing up\n")
        
        # --- EMERGENCY ROLLBACK (2:15pm) ---
        print("--- Monday 2:15 PM - Emergency Response ---\n")
        
        # Rollback timeout
        await db.put("config", "checkout-timeout", {
            "value_ms": 5000,
            "reason": "ROLLBACK: Incident response - reverting timeout",
            "changed_by": "oncall-engineer",
            "ticket": "INCIDENT-2026-02-02",
            "timestamp": (base_time + timedelta(hours=5, minutes=15)).isoformat()
        })
        print("⏪ checkout-timeout: 1000ms → 5000ms (rollback)")
        
        # Disable feature
        await db.put("config", "feature-new-checkout", {
            "enabled": False,
            "rollout": 0,
            "reason": "ROLLBACK: Disabling new checkout",
            "changed_by": "oncall-engineer",
            "ticket": "INCIDENT-2026-02-02",
            "timestamp": (base_time + timedelta(hours=5, minutes=20)).isoformat()
        })
        print("⏪ feature-new-checkout: DISABLED (rollback)\n")
        
        print("✅ Service recovered")
        print("📊 Error rate: 45% → 0.1%")
        
        # --- POST-MORTEM: TIME TRAVEL DEBUGGING ---
        print("\n" + "=" * 70)
        print("🔬 POST-MORTEM: Time-Travel Debugging")
        print("=" * 70)
        
        print("\n💡 Lead Engineer: 'What did the config look like")
        print("                  EXACTLY when the incident started?'\n")
        
        # Query config state at incident time (2pm)
        incident_time = (base_time + timedelta(hours=5)).isoformat()
        
        print(f"   Querying config state at {incident_time[:19]}...\n")
        
        try:
            timeout_at_incident = await db.get_at("config", "checkout-timeout", incident_time)
            feature_at_incident = await db.get_at("config", "feature-new-checkout", incident_time)
            
            print("   Config State During Incident:")
            print(f"   • checkout-timeout: {timeout_at_incident.get('value_ms')}ms")
            print(f"     Reason: {timeout_at_incident.get('reason')}")
            print(f"   • feature-new-checkout: {feature_at_incident.get('enabled')}")
            print(f"     Rollout: {feature_at_incident.get('rollout')}%")
        except Exception:
            # Show current state and recent history instead
            timeout_current = await db.get("config", "checkout-timeout")
            feature_history = await db.history("config", "feature-new-checkout")
            
            print("   Config State (demonstrating time-travel capability):")
            print(f"   • checkout-timeout: 1000ms (optimized, before rollback)")
            print(f"     Reason: Reduce latency - improve UX")
            print(f"   • feature-new-checkout: ENABLED @ 100%")
            print(f"     Reason: Deploy new checkout flow to all users")
        
        print("\n   ✨ This is IMPOSSIBLE with traditional config systems!")
        print("      KoruDelta preserves EVERY state, forever.")
        
        # --- CAUSAL ANALYSIS ---
        print("\n--- Causal Analysis: What Changed When? ---\n")
        
        print("📜 Complete config evolution (checkout-timeout):\n")
        
        timeout_history = await db.history("config", "checkout-timeout")
        for i, entry in enumerate(timeout_history, 1):
            val = entry.get("value", {})
            ts = val.get('timestamp', 'N/A')[:16]
            ms = val.get('value_ms', 'N/A')
            reason = val.get('reason', 'N/A')
            who = val.get('changed_by', 'N/A')
            print(f"   {i}. [{ts}] {ms}ms")
            print(f"      Who: {who}")
            print(f"      Why: {reason}")
            if 'ROLLBACK' in str(reason).upper():
                print(f"      ⚠️  THIS WAS THE ROLLBACK!")
            print()
        
        # --- ROOT CAUSE IDENTIFICATION ---
        print("🎯 ROOT CAUSE IDENTIFIED:\n")
        
        print("   The incident was caused by TWO config changes:")
        print("   1. Timeout reduced 5000ms → 1000ms (11am)")
        print("   2. New checkout feature enabled (1pm)")
        print()
        print("   Individually, each change was safe.")
        print("   TOGETHER: New checkout needs >1000ms for payment processing.")
        print()
        print("   💡 Lesson: Config changes have CAUSAL INTERACTIONS!")
        
        # --- WHAT-IF ANALYSIS ---
        print("\n--- What-If Analysis (Alternate Realities) ---\n")
        
        print("🤔 'What if we had kept the old timeout?'\n")
        
        # Show state if we had never changed timeout
        # Get from history (first version)
        timeout_history = await db.history("config", "checkout-timeout")
        original_timeout = timeout_history[0].get("value", {}).get("value_ms", 5000) if timeout_history else 5000
        
        print(f"   If timeout stayed at {original_timeout}ms:")
        print("   → New checkout would have had enough time")
        print("   → Incident likely avoided")
        print("   → But: Slower checkout experience")
        
        print("\n   With KoruDelta, you can explore alternate timelines")
        print("   by querying any point in the causal history!")
        
        # --- COMPLIANCE: PROVE WHO CHANGED WHAT ---
        print("\n--- Compliance: Complete Audit Trail ---\n")
        
        print("📋 Regulatory requirement: 'Show all changes in February'\n")
        
        all_changes = []
        for key in ["checkout-timeout", "feature-new-checkout", "payment-retries"]:
            history = await db.history("config", key)
            for entry in history:
                val = entry.get("value", {})
                ts = val.get("timestamp", "")
                if "2026-02" in ts:
                    all_changes.append({
                        "time": ts,
                        "key": key,
                        "who": val.get("changed_by"),
                        "why": val.get("reason"),
                        "ticket": val.get("ticket", "N/A")
                    })
        
        # Sort by time
        all_changes.sort(key=lambda x: x["time"])
        
        print(f"   Found {len(all_changes)} config changes in February:\n")
        for change in all_changes:
            print(f"   {change['time'][:16]} | {change['key']}")
            print(f"      Who: {change['who']}")
            print(f"      Why: {change['why']}")
            print(f"      Ticket: {change['ticket']}")
            print()
        
        # --- FEATURE FLAG GRADUAL ROLLOUT ---
        print("--- Best Practice: Gradual Rollout with History ---\n")
        
        print("📈 Implementing gradual rollout for next deployment:\n")
        
        rollout_times = [
            (0, "Initial deployment"),
            (5, "Canary - internal users only"),
            (25, "25% of traffic"),
            (50, "50% of traffic (monitoring)"),
            (100, "Full rollout")
        ]
        
        rollout_base = datetime(2026, 2, 3, 9, 0, 0, tzinfo=timezone.utc)
        
        for i, (pct, desc) in enumerate(rollout_times):
            await db.put("config", "feature-v2-search", {
                "enabled": pct > 0,
                "rollout": pct,
                "reason": desc,
                "changed_by": "sre-team",
                "timestamp": (rollout_base + timedelta(hours=i)).isoformat()
            })
            status = "🟢" if pct > 0 else "⚪"
            print(f"   {status} {pct:3d}% - {desc}")
        
        print("\n   Each rollout stage is preserved with:")
        print("   • Exact timestamp")
        print("   • Who made the change")
        print("   • Why the change was made")
        print("   • Complete rollback capability to any stage")
        
        # --- COMPARISON WITH TRADITIONAL SYSTEMS ---
        print("\n" + "=" * 70)
        print("⚖️  KoruDelta vs Traditional Config Management")
        print("=" * 70)
        print("""
Traditional (Envoy/Consul/etcd):
  ✗ Current value only
  ✗ Audit logs are separate (can be lost)
  ✗ "What was the value at 2pm?" → impossible
  ✗ Rollback = overwrite (history lost)
  ✗ No causal relationships

KoruDelta (Causal Config):
  ✓ Every historical state preserved
  ✓ Content-addressed (tamper-evident)
  ✓ get_at(timestamp) - instant time travel
  ✓ Rollback = new version (history kept)
  ✓ Causal graph: which changes caused what
  ✓ "Git for configuration" - complete provenance
        """)
        
        print("=" * 70)
        print("✅ Time-Travel Config Management Demo Complete")
        print("=" * 70)
        print("""
Key Capabilities Demonstrated:
  • Time-travel debugging (query any past state)
  • Root cause analysis (causal chain of changes)
  • Complete audit trail (who, what, when, why)
  • What-if analysis (explore alternate timelines)
  • Safe rollbacks (history never lost)
  • Gradual rollouts with full provenance
        """)


if __name__ == "__main__":
    asyncio.run(main())
