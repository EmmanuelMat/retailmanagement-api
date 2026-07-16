# 05 - Payroll Advance (Adelantos) - Earned Wage Access for DR

## Problem in Dominican SMEs

Colmado employee needs RD$2,000 for medicine on day 12, but quincena is day 15. Boss gives cash from caja, no record, breaks accounting, no ISR/TSS calc.

## Solution: EWA Model Adapted to DR

Inspired by Netchex, DailyPay, Payactiv [1][2][3] but simplified for small business without third-party provider.

**Core Rules:**
- Advance = access to already EARNED wages, not loan [1]
- Max 50% of accrued net per period [1]
- Deduction model: employer deducts from paycheck and recovers [4]
- Fee: RD$0 for MVP, optional RD$25 flat

### Real-Time Accrual Engine

Every time employee clocks in or makes sale with commission, event `HoursAccrued`:

```
accrued_net = (hours * rate) + commissions - taxes_estimated

Tax estimation for advance calc (prevent overdraw - like Tapcheck net vs gross [5]):
  TSS estimated = 5.91% (2.87% AFP + 3.04% SCF)
  ISR estimated: progressive per DGII tables, simplified 0% if < RD$52k/mo
  net = gross * (1 - tss_pct) - isr_estimate

Available for advance = accrued_net - advance_balance_taken - min_reserve (RD$1000)
```

Projector updates `read_employee_balances` in real-time.

### Flow in Rust Core

```rust
pub fn handle_request_advance(
  employee: &EmployeeAggregate,
  cmd: RequestAdvance { amount: Decimal, reason: String }
) -> Result<Vec<Event>, Error> {
  if amount <= 0 { return Err(InvalidAmount) }
  let available = employee.accrued_net - employee.advance_balance;
  if amount > available * dec!(0.50) {
    return Err(Exceeds50Percent)
  }
  if employee.advance_balance + amount > employee.max_advance_per_period {
    return Err(ExceedsPeriodLimit)
  }
  // Two-phase TigerBeetle reserve
  let transfer_id = Uuid::new_v4().as_u128();
  // Create pending transfer: Debit anticipos_empleados, Credit caja
  // If reserve fails (no cash), reject
  Ok(vec![
    AdvanceRequested { amount, request_id: transfer_id, reason: cmd.reason },
  ])
}

// After TigerBeetle pending success, async approval (could require manager approval in UI)
pub fn approve_advance(...) -> Event {
  // Post pending transfer
  AdvanceApproved { amount, transfer_id, approved_by }
}
```

### Accounting (Double-Entry)

**Day 12 - Advance RD$3000:**
```
Account: asset:anticipos_empleados:{employee_id}
  TigerBeetle:
    id: 3001, debit_account: 5 (anticipos), credit_account: 1 (caja), amount: 300000 (cents), flags: pending
  Then post -> posted

  Journal for GL (Postgres):
    Debit: 1-05 Anticipos a Empleados 3,000
    Credit: 1-01 Caja 3,000
```

**Day 15 - Payroll Run RD$20k gross, deductions TSS 590, ISR 1000, advance 3000:**

```
Transfers (linked atomic, all or none):
  1. Debit expense:sueldos 20,000 | Credit liability:tss_por_pagar 590
  2. Debit expense:sueldos | Credit liability:isr 1000
  3. Debit expense:sueldos | Credit asset:anticipos_empleados 3000 (settles)
  4. Debit expense:sueldos | Credit asset:banco 15410

If employee resigns with pending advance not covered by last payroll:
  Create account receivable: employee loan -> deduct from liquidation.
```

### Compliance DR

- **TSS:** Advance not subject to TSS contributions (it's not new income, it's early payment). Payroll ISR still calculated on gross.
- **ISR:** Advance not taxable extra, but must appear on payroll as deduction "Anticipo de Salario"
- **Receipt:** Paystub must show: Salario Bruto 20k, Deducciones: TSS 590, ISR 1000, Anticipo 3000, Neto 15410
- **Balancing:** Year-end, sum of all AdvanceApproved must equal sum of deductions in PayrollExecuted. Easy to audit via event log.

### UI Flow (Next.js)

1. Employee in mobile app sees "Disponible para adelanto: RD$4,500 (de RD$9,000 ganados)"
2. Clicks "Solicitar RD$2,000" -> Reason "Medicina"
3. Manager gets push notification in web POS dashboard, approves
4. Rust core posts transfer, employee gets notification + cash from caja
5. On payroll screen, advance appears auto-deducted.

### Edge Cases

- Employee requests advance but quits next day: Void pending TigerBeetle transfer, or create loan receivable.
- Multiple advances per period: sum must stay <= 50%
- Negative accrual (hours correction): If accrued_net becomes negative after correction, next payroll deducts.
- Auditor asks proof: Show event chain HoursAccrued -> AdvanceRequested -> AdvanceApproved -> PayrollExecuted with hashes.

### Future: Integration with Banking

Later, instead of cash from caja, transfer via Banco Popular API to employee account. Same ledger entry, just different credit account (banco vs caja).

References:
- Netchex FlexPay real-time accrual [1]
- DailyPay non-borrow model [2]
- Tapcheck gross vs net [5]
- Payactiv factoring legal model [6]
- Deduction models [4]
