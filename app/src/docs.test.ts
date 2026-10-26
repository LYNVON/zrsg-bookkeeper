import { describe, expect, it } from 'vitest';
import { renderReceipt, renderLaborContract, renderReimbursement } from './docs';

describe('文书模板', () => {
  it('收据包含公司、收款人、金额与日期', () => {
    const html = renderReceipt({
      name: '张三',
      idCard: '440101199001011234',
      reason: '临时用工',
      amount: 800,
      date: '2026-10-26',
      company: '自然拾光科技（广州）有限公司',
    } as never);
    expect(html).toContain('自然拾光科技（广州）有限公司');
    expect(html).toContain('张三');
    expect(html).toContain('800');
    expect(html).toContain('2026-10-26');
  });

  it('劳务协议包含工期与总额', () => {
    const html = renderLaborContract({
      company: '甲',
      taxNo: 'T1',
      worker: '李四',
      idCard: '440101199001011234',
      title: '搬运',
      days: '2026-10-01',
      total: 1200,
      date: '2026-10-02',
    } as never);
    expect(html).toContain('李四');
    expect(html).toContain('1200');
    expect(html).toContain('搬运');
  });

  it('报销单合计与明细行数一致', () => {
    const html = renderReimbursement({
      title: '票夹归集报销',
      applicant: '王五',
      formDate: '2026-10-26',
      lines: [
        { expense_on: '2026-10-01', category: '餐饮', invoice_no: '24312000000000000001', amount: 88.5 },
        { expense_on: '2026-10-02', category: '交通', invoice_no: '24312000000000000002', amount: 12 },
      ],
    } as never);
    expect(html).toContain('王五');
    expect(html).toContain('餐饮');
    expect(html).toContain('100.5');
  });
});
