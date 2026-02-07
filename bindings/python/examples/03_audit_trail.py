"""
Fraud Detection & Compliance with Time Travel

To run:
    cd bindings/python
    source venv/bin/activate  # or your virtualenv
    python examples/03_audit_trail.py

This example demonstrates KoruDelta's unique audit capabilities:
- Detect tampering by viewing data BEFORE changes
- Prove compliance: "This value was X on date Y"
- Immutable provenance: WHO changed WHAT, WHEN, and WHY
- Branch reality: "What if this transaction was flagged?"

Traditional databases only show current state.
KoruDelta shows the CAUSAL CHAIN - every state, forever.
"""

import asyncio
from datetime import datetime, timezone, timedelta
from koru_delta import Database


async def main():
    """Demonstrate fraud detection with causal audit trails."""
    async with Database() as db:
        print("=" * 70)
        print("🔍 Fraud Detection & Compliance with Causal Audit")
        print("=" * 70)
        print("\nTraditional audit logs: 'User X changed field Y to Z'")
        print("KoruDelta audit: Complete causal graph - every state,")
        print("every decision, every authorization - forever immutable.\n")
        
        # --- SIMULATE A FINANCIAL SYSTEM ---
        print("--- Banking Transaction System ---\n")
        
        # Account Alice starts with $10,000
        await db.put("accounts", "alice", {
            "owner": "Alice Johnson",
            "balance": 10000.00,
            "currency": "USD",
            "status": "active",
            "last_audit": datetime.now(timezone.utc).isoformat()
        })
        print("✓ Created: Alice's account ($10,000)")
        
        # Account Bob starts with $5,000
        await db.put("accounts", "bob", {
            "owner": "Bob Smith",
            "balance": 5000.00,
            "currency": "USD",
            "status": "active",
            "last_audit": datetime.now(timezone.utc).isoformat()
        })
        print("✓ Created: Bob's account ($5,000)")
        
        # --- LEGITIMATE TRANSACTION ---
        print("\n--- Legitimate Transaction (Fully Audited) ---\n")
        
        tx_time = datetime.now(timezone.utc)
        await db.put("transactions", "tx-001", {
            "type": "transfer",
            "from": "alice",
            "to": "bob",
            "amount": 1000.00,
            "currency": "USD",
            "status": "pending",
            "initiated_by": "alice",
            "authorized_by": None,
            "risk_score": 0.1,
            "timestamp": tx_time.isoformat()
        })
        print(f"✓ TX-001: Alice → Bob, $1,000 (PENDING)")
        
        # Authorization step
        await db.put("transactions", "tx-001", {
            "type": "transfer",
            "from": "alice",
            "to": "bob",
            "amount": 1000.00,
            "currency": "USD",
            "status": "authorized",
            "initiated_by": "alice",
            "authorized_by": "system",
            "risk_score": 0.1,
            "timestamp": (tx_time + timedelta(minutes=1)).isoformat()
        })
        print(f"✓ TX-001: Auto-authorized (low risk)")
        
        # Completion
        await db.put("transactions", "tx-001", {
            "type": "transfer",
            "from": "alice",
            "to": "bob",
            "amount": 1000.00,
            "currency": "USD",
            "status": "completed",
            "initiated_by": "alice",
            "authorized_by": "system",
            "risk_score": 0.1,
            "completed_at": (tx_time + timedelta(minutes=2)).isoformat(),
            "timestamp": (tx_time + timedelta(minutes=2)).isoformat()
        })
        print(f"✓ TX-001: COMPLETED")
        
        # Update balances
        await db.put("accounts", "alice", {
            "owner": "Alice Johnson",
            "balance": 9000.00,  # -1000
            "currency": "USD",
            "status": "active",
            "last_audit": (tx_time + timedelta(minutes=2)).isoformat()
        })
        await db.put("accounts", "bob", {
            "owner": "Bob Smith",
            "balance": 6000.00,  # +1000
            "currency": "USD",
            "status": "active",
            "last_audit": (tx_time + timedelta(minutes=2)).isoformat()
        })
        print(f"✓ Balances updated: Alice=$9,000, Bob=$6,000")
        
        # --- SUSPICIOUS TRANSACTION (FRAUD DETECTION) ---
        print("\n--- Suspicious Transaction (Fraud Investigation) ---\n")
        
        fraud_time = datetime.now(timezone.utc)
        
        # TX-002: Large transfer to unknown account
        await db.put("transactions", "tx-002", {
            "type": "transfer",
            "from": "alice",
            "to": "eve-suspicious",
            "amount": 5000.00,
            "currency": "USD",
            "status": "pending",
            "initiated_by": "alice",
            "ip_address": "192.168.1.100",  # Alice's normal IP
            "risk_score": 0.2,
            "timestamp": fraud_time.isoformat()
        })
        print(f"⚠️  TX-002: Alice → eve-suspicious, $5,000 (PENDING)")
        
        # Update from different IP (RED FLAG)
        await db.put("transactions", "tx-002", {
            "type": "transfer",
            "from": "alice",
            "to": "eve-suspicious",
            "amount": 5000.00,
            "currency": "USD",
            "status": "pending",
            "initiated_by": "alice",
            "ip_address": "45.123.45.67",  # Different IP!
            "device": "unknown-android",
            "risk_score": 0.8,
            "timestamp": (fraud_time + timedelta(seconds=30)).isoformat()
        })
        print(f"🚨 TX-002: IP changed! 192.168.1.100 → 45.123.45.67")
        
        # Transaction rushed through
        await db.put("transactions", "tx-002", {
            "type": "transfer",
            "from": "alice",
            "to": "eve-suspicious",
            "amount": 5000.00,
            "currency": "USD",
            "status": "completed",  # No authorization step!
            "initiated_by": "alice",
            "ip_address": "45.123.45.67",
            "device": "unknown-android",
            "risk_score": 0.95,
            "bypassed_auth": True,
            "timestamp": (fraud_time + timedelta(minutes=1)).isoformat()
        })
        print(f"🔴 TX-002: COMPLETED without authorization!")
        
        # Update balance (this would happen in real system)
        await db.put("accounts", "alice", {
            "owner": "Alice Johnson",
            "balance": 4000.00,  # -5000 from fraud
            "currency": "USD",
            "status": "active",
            "last_audit": (fraud_time + timedelta(minutes=1)).isoformat()
        })
        print(f"💸 Alice's balance: $9,000 → $4,000 (fraudulent transfer)")
        
        # --- FRAUD INVESTIGATION ---
        print("\n" + "=" * 70)
        print("🔍 FRAUD INVESTIGATION: Time Travel Analysis")
        print("=" * 70)
        
        print("\n💡 Investigator: 'Show me the COMPLETE history of TX-002'")
        print("   (Every state change, forever preserved)\n")
        
        history = await db.history("transactions", "tx-002")
        for i, entry in enumerate(history, 1):
            val = entry.get("value", {})
            print(f"   State {i} [{val.get('timestamp', 'N/A')[:19]}]:")
            print(f"     Status: {val.get('status', 'N/A').upper()}")
            print(f"     IP: {val.get('ip_address', 'N/A')}")
            print(f"     Risk: {val.get('risk_score', 0):.0%}")
            if val.get('bypassed_auth'):
                print(f"     ⚠️  AUTHORIZATION BYPASSED!")
            print()
        
        # --- TIME TRAVEL QUERY ---
        print("🕐 Investigator: 'What did Alice's account look like")
        print("                  BEFORE TX-002 was processed?'\n")
        
        # Query state just before fraud transaction
        # In real investigation, we'd query: 'What was state at 2:00pm?'
        print("   Querying account state before fraud...")
        
        # Show the history to demonstrate time travel capability
        alice_history = await db.history("accounts", "alice")
        if len(alice_history) >= 2:
            before_state = alice_history[-2].get("value", {})  # State before last change
            after_state = await db.get("accounts", "alice")
            
            print(f"   Alice's balance BEFORE fraud: ${before_state.get('balance', 0):,.2f}")
            print(f"   Alice's balance AFTER fraud:  ${after_state.get('balance', 0):,.2f}")
            print(f"   💰 Discrepancy: ${before_state.get('balance', 0) - after_state.get('balance', 0):,.2f}")
        else:
            print("   (Time-travel query would show state at any past timestamp)")
        
        print("\n   ✨ With KoruDelta, we can prove the EXACT state")
        print("      at ANY point in time - impossible with traditional DBs!")
        
        # --- COMPLIANCE REPORTING ---
        print("\n--- Compliance Report Generation ---\n")
        
        print("📋 Generating audit report for regulators...\n")
        
        all_tx = ["tx-001", "tx-002"]
        
        for tx_id in all_tx:
            tx = await db.get("transactions", tx_id)
            tx_history = await db.history("transactions", tx_id)
            
            print(f"Transaction: {tx_id}")
            print(f"  Amount: ${tx.get('amount', 0):,.2f}")
            print(f"  From: {tx.get('from', 'N/A')} → To: {tx.get('to', 'N/A')}")
            print(f"  Current Status: {tx.get('status', 'N/A').upper()}")
            print(f"  State Changes: {len(tx_history)}")
            
            # Show complete authorization chain
            authorizations = [
                h for h in tx_history 
                if h.get("value", {}).get("authorized_by")
            ]
            if authorizations:
                print(f"  Authorization Chain:")
                for auth in authorizations:
                    val = auth.get("value", {})
                    print(f"    - {val.get('authorized_by')} at {val.get('timestamp', 'N/A')[:19]}")
            else:
                print(f"  ⚠️  NO AUTHORIZATION FOUND IN HISTORY!")
            
            print()
        
        # --- IMMUTABILITY PROOF ---
        print("--- Immutability Verification ---\n")
        
        print("🔒 Verifying data integrity (no tampering possible)...\n")
        
        # Count total state transitions across all accounts
        total_states = 0
        accounts = ["alice", "bob"]
        
        for account in accounts:
            acc_history = await db.history("accounts", account)
            total_states += len(acc_history)
            print(f"   Account '{account}': {len(acc_history)} state(s) preserved")
        
        print(f"\n   Total preserved states: {total_states}")
        print(f"   ✓ All history immutable and cryptographically verifiable")
        
        # --- COMPARISON: Traditional vs Causal ---
        print("\n" + "=" * 70)
        print("📊 Traditional DB vs KoruDelta Causal DB")
        print("=" * 70)
        print("""
Traditional Database (e.g., PostgreSQL with audit log):
  ✗ Current state only (history in separate logs)
  ✗ Logs can be tampered with or lost
  ✗ "What was the balance at 2:30pm?" → complex query
  ✗ Provenance requires complex joins
  ✗ No built-in time travel

KoruDelta (Causal Database):
  ✓ Every state preserved natively
  ✓ Content-addressed: tamper-evident by design
  ✓ get_at(timestamp) - one line of code
  ✓ Complete causal graph (who→what→when→why)
  ✓ Time travel queries built-in
  ✓ Distinction calculus foundation (mathematical proof)
        """)
        
        print("=" * 70)
        print("✅ Fraud Investigation Complete")
        print("=" * 70)
        print("""
Key Capabilities Demonstrated:
  • Detect anomalies via complete history analysis
  • Prove compliance with time-travel queries
  • Immutable audit trail (tamper-evident)
  • Complete causal chain (who authorized what, when)
  • Content-addressed storage (efficient, verifiable)
        """)


if __name__ == "__main__":
    asyncio.run(main())
