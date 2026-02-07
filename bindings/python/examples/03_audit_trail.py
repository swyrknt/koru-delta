"""
Audit Trail & Compliance Example

Demonstrates KoruDelta's audit capabilities:
- Complete provenance tracking
- Time travel queries
- Immutable history
- Compliance reporting
"""

import asyncio
from datetime import datetime, timezone
from koru_delta import Database


async def main():
    """Demonstrate audit trail capabilities."""
    async with Database() as db:
        print("=" * 60)
        print("Audit Trail & Compliance Demo")
        print("=" * 60)
        
        # Simulate a financial transaction system
        print("\n--- Recording Transactions ---")
        
        # Record initial transaction
        await db.put("transactions", "tx-001", {
            "type": "transfer",
            "from": "account-A",
            "to": "account-B",
            "amount": 1000.00,
            "currency": "USD",
            "status": "pending",
            "initiated_by": "user-123",
            "timestamp": datetime.now(timezone.utc).isoformat()
        })
        print("✓ Recorded: tx-001 (pending transfer $1,000)")
        
        # Record approval
        await db.put("transactions", "tx-001", {
            "type": "transfer",
            "from": "account-A",
            "to": "account-B",
            "amount": 1000.00,
            "currency": "USD",
            "status": "approved",
            "initiated_by": "user-123",
            "approved_by": "admin-456",
            "timestamp": datetime.now(timezone.utc).isoformat()
        })
        print("✓ Recorded: tx-001 approved by admin")
        
        # Record completion
        await db.put("transactions", "tx-001", {
            "type": "transfer",
            "from": "account-A",
            "to": "account-B",
            "amount": 1000.00,
            "currency": "USD",
            "status": "completed",
            "initiated_by": "user-123",
            "approved_by": "admin-456",
            "completed_at": datetime.now(timezone.utc).isoformat(),
            "timestamp": datetime.now(timezone.utc).isoformat()
        })
        print("✓ Recorded: tx-001 completed")
        
        # Record a configuration change
        await db.put("config", "max-transfer-limit", {
            "value": 5000,
            "unit": "USD",
            "changed_by": "admin-789",
            "reason": "Quarterly limit review",
            "timestamp": datetime.now(timezone.utc).isoformat()
        })
        print("✓ Recorded: Config change (max transfer limit)")
        
        # --- TIME TRAVEL QUERY ---
        print("\n--- Time Travel Queries ---")
        
        # Get current state
        current = await db.get("transactions", "tx-001")
        print(f"\nCurrent state: {current['status']}")
        
        # Get full history
        print("\nFull history for tx-001:")
        history = await db.history("transactions", "tx-001")
        for i, entry in enumerate(history, 1):
            value = entry.get("value", {})
            print(f"  {i}. {value.get('status', 'unknown')} at {entry.get('timestamp', 'unknown')[:19]}")
        
        # --- COMPLIANCE REPORTING ---
        print("\n--- Compliance Report ---")
        
        # Generate a compliance report
        print("\nTransaction Audit Report:")
        print(f"  Transaction ID: tx-001")
        print(f"  Total state changes: {len(history)}")
        print(f"  Current status: {current['status']}")
        print(f"  Initiated by: {current['initiated_by']}")
        print(f"  Approved by: {current.get('approved_by', 'N/A')}")
        
        # Check for unauthorized changes
        print("\n  State transition log:")
        prev_status = None
        for entry in history:
            value = entry.get("value", {})
            status = value.get("status', 'unknown'")
            if prev_status and status != prev_status:
                print(f"    {prev_status} → {status}")
            prev_status = status
        
        # --- CONFIG VERSIONING ---
        print("\n--- Config Versioning ---")
        
        # Update config multiple times to show versioning
        for i, limit in enumerate([10000, 15000, 20000], 1):
            await db.put("config", "max-transfer-limit", {
                "value": limit,
                "unit": "USD",
                "changed_by": f"admin-{800 + i}",
                "reason": f"Monthly review #{i}",
                "timestamp": datetime.now(timezone.utc).isoformat()
            })
        
        config_history = await db.history("config", "max-transfer-limit")
        print(f"\nConfig 'max-transfer-limit' has {len(config_history)} versions:")
        for entry in config_history[-3:]:  # Show last 3
            value = entry.get("value", {})
            print(f"  ${value.get('value', 'N/A'):,} - {value.get('reason', 'N/A')}")
        
        # --- DATA INTEGRITY ---
        print("\n--- Data Integrity Verification ---")
        
        # Verify all transactions have proper audit trail
        tx_keys = await db.list_keys("transactions")
        print(f"\nTotal transactions: {len(tx_keys)}")
        
        for key in tx_keys:
            tx_history = await db.history("transactions", key)
            tx_current = await db.get("transactions", key)
            print(f"  {key}: {len(tx_history)} history entries, status={tx_current['status']}")
        
        print("\n" + "=" * 60)
        print("✓ Audit Trail Demo Complete!")
        print("=" * 60)
        print("\nKey Features Demonstrated:")
        print("  • Complete change history (immutable)")
        print("  • Time travel queries (get state at any point)")
        print("  • Provenance tracking (who changed what, when)")
        print("  • Compliance reporting (audit trails)")


if __name__ == "__main__":
    asyncio.run(main())
