# PyReverseETL - Simple Guide for Everyone

**No technical knowledge required.**

---

## What It Does

PyReverseETL moves your data from one place to another automatically:

```
Your Data Source → PyReverseETL → Your Destination
                  (automatic)
```

Examples:
- Move customer information from your systems to your data warehouse
- Copy sales records to your analytics platform
- Sync user profiles to your marketing system
- Update product data across all your tools

---

## How to Set It Up

### Step 1: Describe Your Needs (Simple Text File)

Create a file called `sync.yaml`:

```yaml
name: sync_customers
description: Move customer data to warehouse

check_frequency: Every hour
timezone: America/New_York
```

That's it. No coding required.

### Step 2: Let It Run

```bash
python run_sync.py
```

Done! Your data now syncs automatically.

---

## What You Can Customize

### How Often to Check
- Every 5 minutes
- Every hour
- Every day
- Your choice (any interval you want)

### When NOT to Sync
- Skip weekends
- Skip specific days (e.g., Mondays)
- Skip nighttime hours (e.g., 8 PM - 8 AM)
- Skip holiday periods

### Your Time Zone
- Automatically adjusts for your location
- Supports worldwide timezones
- Examples: New York, London, Tokyo, Sydney

### What to Do With The Data
- Copy as-is (no changes)
- Clean it up (fix formatting, normalize data)
- Transform it (combine fields, calculate values)

---

## What You Get

✅ **Automatic syncs** - Runs on schedule, no manual work  
✅ **No data loss** - Everything gets delivered  
✅ **Works all hours** - Runs 24/7 on your schedule  
✅ **Easy setup** - Simple text file configuration  
✅ **See what's happening** - Automatic reports and status  

---

## Real-World Example

**Scenario:** Sync your customer database every hour, but:
- Skip Saturdays and Sundays
- Don't run between 10 PM and 6 AM
- Fix phone number formatting

**Configuration:**

```yaml
name: customer_sync
description: Move customer data every hour

check_frequency: Every hour
timezone: America/New_York

skip_days:
  - Saturday
  - Sunday

no_sync_hours: 10 PM to 6 AM

transform: Clean up phone numbers
```

**Result:** 
- Customers synced Monday-Friday
- During business hours (6 AM - 10 PM)
- With clean phone numbers
- Automatically, without any manual work

---

## Support & Help

### Need Help?
1. Check the simple examples: `/examples` folder
2. Read the detailed guide: `USER_GUIDE_v2.0.1.md`
3. Common issues: See `QUICK_START.md`

### Something Wrong?
Look for error messages in the output. The system tells you exactly what's wrong and how to fix it.

---

## Summary

PyReverseETL makes data syncing:
- **Simple:** One text file to configure
- **Reliable:** Gets data where it needs to go
- **Flexible:** Syncs on your schedule
- **Hands-off:** Runs automatically

No technical expertise needed. Just set it and forget it.

---

**Ready to get started? Create a `sync.yaml` file and run `python run_sync.py`**
