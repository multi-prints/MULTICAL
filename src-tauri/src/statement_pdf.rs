//! Generate a bank-ready business revenue statement as a PDF (A4).
//!
//! Layout is guided by official statement PDFs (summary tables, section titles,
//! meta header, footer band) but uses MULTIPRINTS branding — not third-party styles.

use printpdf::path::{PaintMode, WindingOrder};
use printpdf::*;
use std::io::BufWriter;

use crate::models::BusinessStatement;

// MULTIPRINTS brand (app theme) — not Safaricom green
const BRAND: (f32, f32, f32) = (101.0 / 255.0, 101.0 / 255.0, 236.0 / 255.0); // #6565EC
const BRAND_DARK: (f32, f32, f32) = (82.0 / 255.0, 82.0 / 255.0, 212.0 / 255.0); // #5252D4
const BRAND_SOFT: (f32, f32, f32) = (238.0 / 255.0, 238.0 / 255.0, 252.0 / 255.0);
const TEXT: (f32, f32, f32) = (23.0 / 255.0, 23.0 / 255.0, 23.0 / 255.0);
const MUTED: (f32, f32, f32) = (82.0 / 255.0, 82.0 / 255.0, 82.0 / 255.0);
const GRID: (f32, f32, f32) = (212.0 / 255.0, 212.0 / 255.0, 212.0 / 255.0);
const ROW_ALT: (f32, f32, f32) = (248.0 / 255.0, 248.0 / 255.0, 252.0 / 255.0);
const WHITE: (f32, f32, f32) = (1.0, 1.0, 1.0);
const STAMP_BORDER: (f32, f32, f32) = (82.0 / 255.0, 82.0 / 255.0, 212.0 / 255.0);

const PAGE_W: f32 = 210.0;
const PAGE_H: f32 = 297.0;
const MARGIN: f32 = 14.0;
const FOOTER_H: f32 = 20.0;
const FOOTER_Y0: f32 = 8.0;
/// Content must stay above the footer band.
const CONTENT_BOTTOM: f32 = FOOTER_Y0 + FOOTER_H + 6.0;
const CONTENT_TOP: f32 = PAGE_H - 16.0;

fn rgb(c: (f32, f32, f32)) -> Color {
    Color::Rgb(Rgb::new(c.0, c.1, c.2, None))
}

fn money_plain(v: f64) -> String {
    let n = v.round() as i64;
    let neg = n < 0;
    let s = n.unsigned_abs().to_string();
    let mut out = String::new();
    for (i, ch) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    let digits: String = out.chars().rev().collect();
    if neg {
        format!("-{digits}")
    } else {
        digits
    }
}

fn method_label(m: &str) -> String {
    match m.to_lowercase().as_str() {
        "mpesa" | "m-pesa" => "M-Pesa".into(),
        "cash" => "Cash".into(),
        "till" => "Till".into(),
        "bank" => "Bank".into(),
        other => {
            let mut c = other.chars();
            match c.next() {
                None => other.to_string(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        }
    }
}

fn source_label(source: &str) -> &'static str {
    match source {
        "sales" => "Product & stock sales",
        "printing" => "Printing services",
        _ => "Sales and printing",
    }
}

fn source_code(source: &str) -> &'static str {
    match source {
        "sales" => "SAL",
        "printing" => "PRT",
        _ => "ALL",
    }
}

fn fmt_date_iso(iso: &str) -> String {
    let parts: Vec<&str> = iso.split('-').collect();
    if parts.len() != 3 {
        return iso.to_string();
    }
    let month = match parts[1] {
        "01" => "Jan",
        "02" => "Feb",
        "03" => "Mar",
        "04" => "Apr",
        "05" => "May",
        "06" => "Jun",
        "07" => "Jul",
        "08" => "Aug",
        "09" => "Sep",
        "10" => "Oct",
        "11" => "Nov",
        "12" => "Dec",
        _ => parts[1],
    };
    let day = parts[2].trim_start_matches('0');
    let day = if day.is_empty() { parts[2] } else { day };
    format!("{} {} {}", day, month, parts[0])
}

/// Approximate Helvetica string width in mm (slightly generous to avoid right-edge clip).
fn text_width_mm(text: &str, size_pt: f32) -> f32 {
    // ~0.55em average for mixed-case Helvetica; pt → mm = 0.352778
    text.chars().count() as f32 * size_pt * 0.55 * 0.352778
}

fn fill_rect(layer: &PdfLayerReference, x0: Mm, y0: Mm, x1: Mm, y1: Mm, color: (f32, f32, f32)) {
    layer.set_fill_color(rgb(color));
    layer.add_rect(
        Rect::new(x0, y0, x1, y1)
            .with_mode(PaintMode::Fill)
            .with_winding(WindingOrder::NonZero),
    );
}

fn stroke_rect(
    layer: &PdfLayerReference,
    x0: Mm,
    y0: Mm,
    x1: Mm,
    y1: Mm,
    color: (f32, f32, f32),
    thickness: f32,
) {
    layer.set_outline_color(rgb(color));
    layer.set_outline_thickness(thickness);
    layer.add_rect(
        Rect::new(x0, y0, x1, y1)
            .with_mode(PaintMode::Stroke)
            .with_winding(WindingOrder::NonZero),
    );
}

fn hline(layer: &PdfLayerReference, x0: Mm, x1: Mm, y: Mm, color: (f32, f32, f32), thickness: f32) {
    layer.set_outline_color(rgb(color));
    layer.set_outline_thickness(thickness);
    layer.add_line(Line {
        points: vec![(Point::new(x0, y), false), (Point::new(x1, y), false)],
        is_closed: false,
    });
}

fn text_at(
    layer: &PdfLayerReference,
    s: &str,
    size: f32,
    x: Mm,
    y: Mm,
    font: &IndirectFontRef,
    color: (f32, f32, f32),
) {
    layer.set_fill_color(rgb(color));
    layer.use_text(s, size, x, y, font);
}

fn text_center(
    layer: &PdfLayerReference,
    s: &str,
    size: f32,
    y: Mm,
    font: &IndirectFontRef,
    color: (f32, f32, f32),
) {
    let w = text_width_mm(s, size);
    let x = ((PAGE_W - w) / 2.0).max(MARGIN);
    text_at(layer, s, size, Mm(x), y, font, color);
}

struct TableCol {
    title: &'static str,
    x: f32,
    w: f32,
    right: bool,
}

struct PdfCtx<'a> {
    doc: &'a PdfDocumentReference,
    font: &'a IndirectFontRef,
    font_bold: &'a IndirectFontRef,
    pages: Vec<(PdfPageIndex, PdfLayerIndex)>,
    y: f32,
    left: f32,
    right: f32,
    stmt: &'a BusinessStatement,
    ref_code: String,
}

impl<'a> PdfCtx<'a> {
    fn layer(&self) -> PdfLayerReference {
        let (pi, li) = *self.pages.last().unwrap();
        self.doc.get_page(pi).get_layer(li)
    }

    fn page_no(&self) -> usize {
        self.pages.len()
    }

    fn draw_footer_and_page_no(&self, page_idx: usize, total_pages: usize) {
        let (pi, li) = self.pages[page_idx];
        let layer = self.doc.get_page(pi).get_layer(li);

        // Page number top-right
        text_at(
            &layer,
            &format!("Page {} of {}", page_idx + 1, total_pages),
            8.0,
            Mm(self.right - 24.0),
            Mm(PAGE_H - 10.0),
            self.font,
            MUTED,
        );

        // Soft brand footer strip
        fill_rect(
            &layer,
            Mm(0.0),
            Mm(FOOTER_Y0),
            Mm(PAGE_W),
            Mm(FOOTER_Y0 + FOOTER_H),
            BRAND_SOFT,
        );
        hline(
            &layer,
            Mm(0.0),
            Mm(PAGE_W),
            Mm(FOOTER_Y0 + FOOTER_H),
            BRAND,
            1.0,
        );

        text_at(
            &layer,
            "Statement reference",
            7.0,
            Mm(self.left),
            Mm(FOOTER_Y0 + FOOTER_H - 6.0),
            self.font,
            MUTED,
        );
        text_at(
            &layer,
            &self.ref_code,
            11.0,
            Mm(self.left),
            Mm(FOOTER_Y0 + 4.5),
            self.font_bold,
            TEXT,
        );

        let brand = "MULTIPRINTS";
        let bw = text_width_mm(brand, 10.0);
        text_at(
            &layer,
            brand,
            10.0,
            Mm(self.right - bw),
            Mm(FOOTER_Y0 + 7.0),
            self.font_bold,
            BRAND,
        );
    }

    fn new_page(&mut self) {
        let (p, l) = self.doc.add_page(Mm(PAGE_W), Mm(PAGE_H), "Layer 1");
        self.pages.push((p, l));
        self.y = CONTENT_TOP - 4.0;
    }

    /// Ensure `needed` mm of vertical space; open a new page if required.
    fn ensure(&mut self, needed: f32) {
        if self.y - needed < CONTENT_BOTTOM {
            self.new_page();
        }
    }

    /// Section title — reserves space for the following table so titles are not orphaned.
    fn section_title(&mut self, title: &str, following_h: f32) {
        self.ensure(12.0 + following_h);
        let layer = self.layer();
        text_center(&layer, title, 11.0, Mm(self.y), self.font_bold, BRAND);
        self.y -= 4.5;
    }

    fn table_height(row_count: usize) -> f32 {
        6.8 + 5.8 * row_count as f32 + 2.0
    }

    fn draw_table(&mut self, cols: &[TableCol], rows: &[Vec<String>], emphasize_last: bool) {
        let header_h = 6.8_f32;
        let row_h = 5.8_f32;
        let table_h = header_h + row_h * rows.len() as f32;
        self.ensure(table_h + 2.0);

        let layer = self.layer();
        let top_y = self.y;
        let mut y = top_y;

        // Header
        fill_rect(
            &layer,
            Mm(self.left),
            Mm(y - header_h),
            Mm(self.right),
            Mm(y),
            BRAND,
        );
        for col in cols {
            let size = 7.5_f32;
            // Headers are left-padded in each column (avoids right-edge clip from font metrics).
            // Numeric body cells stay right-aligned below.
            let tx = if col.right {
                // Sit near the right side with a conservative inset
                (col.x + col.w - text_width_mm(col.title, size) * 1.25 - 2.0).max(col.x + 1.0)
            } else {
                col.x + 1.5
            };
            text_at(
                &layer,
                col.title,
                size,
                Mm(tx),
                Mm(y - header_h + 1.9),
                self.font_bold,
                WHITE,
            );
        }
        y -= header_h;

        for (i, row) in rows.iter().enumerate() {
            let is_last = emphasize_last && i + 1 == rows.len();
            let bg = if is_last {
                BRAND_SOFT
            } else if i % 2 == 1 {
                ROW_ALT
            } else {
                WHITE
            };
            fill_rect(
                &layer,
                Mm(self.left),
                Mm(y - row_h),
                Mm(self.right),
                Mm(y),
                bg,
            );

            for (ci, cell) in row.iter().enumerate() {
                if ci >= cols.len() {
                    break;
                }
                let col = &cols[ci];
                let f = if is_last { self.font_bold } else { self.font };
                let size = if is_last { 8.0 } else { 7.5 };
                // Truncate visually long cells so they stay inside the column
                let mut display = cell.clone();
                while text_width_mm(&display, size) > col.w - 3.0 && display.len() > 4 {
                    display.pop();
                }
                if display.len() < cell.len() {
                    display.push('…');
                }
                let tx = if col.right {
                    (col.x + col.w - text_width_mm(&display, size) - 1.2).max(col.x + 0.5)
                } else {
                    col.x + 1.5
                };
                text_at(&layer, &display, size, Mm(tx), Mm(y - row_h + 1.6), f, TEXT);
            }
            y -= row_h;
        }

        // Border + grid
        stroke_rect(
            &layer,
            Mm(self.left),
            Mm(y),
            Mm(self.right),
            Mm(top_y),
            GRID,
            0.55,
        );
        let mut hy = top_y - header_h;
        hline(&layer, Mm(self.left), Mm(self.right), Mm(hy), GRID, 0.45);
        for _ in 0..rows.len() {
            hy -= row_h;
            hline(&layer, Mm(self.left), Mm(self.right), Mm(hy), GRID, 0.3);
        }
        for col in cols.iter().skip(1) {
            layer.set_outline_color(rgb(GRID));
            layer.set_outline_thickness(0.35);
            layer.add_line(Line {
                points: vec![
                    (Point::new(Mm(col.x), Mm(y)), false),
                    (Point::new(Mm(col.x), Mm(top_y)), false),
                ],
                is_closed: false,
            });
        }

        self.y = y - 7.0;
    }

    fn two_col_table(&self) -> [TableCol; 2] {
        let tw = self.right - self.left;
        // Wider amount column so "AMOUNT (KSh)" never clips
        let c0 = tw * 0.58;
        let c1 = tw * 0.42;
        [
            TableCol {
                title: "CATEGORY",
                x: self.left,
                w: c0,
                right: false,
            },
            TableCol {
                title: "AMOUNT",
                x: self.left + c0,
                w: c1,
                right: true,
            },
        ]
    }
}

/// Build PDF bytes for a business revenue statement.
pub fn render_business_statement_pdf(stmt: &BusinessStatement) -> Result<Vec<u8>, String> {
    let left = MARGIN;
    let right = PAGE_W - MARGIN;

    let (doc, page1, layer1) = PdfDocument::new(
        "MULTIPRINTS Business Revenue Statement",
        Mm(PAGE_W),
        Mm(PAGE_H),
        "Layer 1",
    );

    let font = doc
        .add_builtin_font(BuiltinFont::Helvetica)
        .map_err(|e| format!("PDF font: {e}"))?;
    let font_bold = doc
        .add_builtin_font(BuiltinFont::HelveticaBold)
        .map_err(|e| format!("PDF bold font: {e}"))?;

    let ref_code = format!(
        "MP-{}-{}M-{}",
        stmt.period_end.replace('-', ""),
        stmt.months,
        source_code(&stmt.source)
    );

    let mut ctx = PdfCtx {
        doc: &doc,
        font: &font,
        font_bold: &font_bold,
        pages: vec![(page1, layer1)],
        y: CONTENT_TOP - 6.0,
        left,
        right,
        stmt,
        ref_code,
    };

    // ---- Title ----
    {
        let layer = ctx.layer();
        text_center(
            &layer,
            "BUSINESS REVENUE STATEMENT",
            13.5,
            Mm(ctx.y),
            ctx.font_bold,
            TEXT,
        );
        ctx.y -= 5.0;
        text_center(&layer, "MULTIPRINTS", 9.0, Mm(ctx.y), ctx.font_bold, BRAND);
        ctx.y -= 8.0;
    }

    // ---- Meta + stamp ----
    {
        let layer = ctx.layer();
        let meta_top = ctx.y;
        let meta_pairs = [
            (
                "Requested by:",
                stmt.requested_by.clone().unwrap_or_else(|| "admin".into()),
            ),
            ("Data included:", source_label(&stmt.source).to_string()),
            (
                "Statement period:",
                format!(
                    "{} - {} ({} mo)",
                    fmt_date_iso(&stmt.period_start),
                    fmt_date_iso(&stmt.period_end),
                    stmt.months
                ),
            ),
            ("Generated:", stmt.generated_at.clone()),
            ("Application:", format!("MULTIPRINTS v{}", stmt.app_version)),
        ];

        let mut my = meta_top;
        for (label, value) in &meta_pairs {
            text_at(&layer, label, 8.0, Mm(left), Mm(my), ctx.font, BRAND);
            text_at(&layer, value, 8.0, Mm(left + 36.0), Mm(my), ctx.font, TEXT);
            my -= 4.8;
        }

        let stamp_x0 = 126.0;
        let stamp_x1 = right;
        let stamp_y1 = meta_top + 1.0;
        let stamp_y0 = stamp_y1 - 26.0;
        stroke_rect(
            &layer,
            Mm(stamp_x0),
            Mm(stamp_y0),
            Mm(stamp_x1),
            Mm(stamp_y1),
            STAMP_BORDER,
            1.15,
        );
        let stamp_mid = (stamp_x0 + stamp_x1) / 2.0;
        for (line, size, color, dy) in [
            ("OFFICIAL BUSINESS RECORDS", 7.2_f32, BRAND_DARK, 7.5_f32),
            (
                &format!(
                    "{} – {}",
                    fmt_date_iso(&stmt.period_start),
                    fmt_date_iso(&stmt.period_end)
                ) as &str,
                6.8,
                MUTED,
                13.5,
            ),
            ("Generated from MULTIPRINTS", 7.0, BRAND, 19.5),
        ] {
            let w = text_width_mm(line, size);
            text_at(
                &layer,
                line,
                size,
                Mm(stamp_mid - w / 2.0),
                Mm(stamp_y1 - dy),
                if color == MUTED {
                    ctx.font
                } else {
                    ctx.font_bold
                },
                color,
            );
        }

        ctx.y = my.min(stamp_y0) - 7.0;
    }

    // ---- SUMMARY ----
    {
        let rows = vec![
            vec![
                "Total cash collected".into(),
                money_plain(stmt.total_cash_collected),
            ],
            vec![
                "Total gross billed (incl. credit sales)".into(),
                money_plain(stmt.total_gross_billed),
            ],
            vec![
                "Number of transactions / jobs".into(),
                stmt.total_transactions.to_string(),
            ],
            vec![
                "Average monthly cash collected".into(),
                money_plain(stmt.average_monthly_cash),
            ],
            vec![
                "Outstanding receivables (period debts)".into(),
                money_plain(stmt.period_outstanding_receivables),
            ],
            vec![
                "TOTAL CASH COLLECTED".into(),
                money_plain(stmt.total_cash_collected),
            ],
        ];
        ctx.section_title("SUMMARY", PdfCtx::table_height(rows.len()) + 4.0);
        // Currency note under section title (amounts column is KSh)
        {
            let layer = ctx.layer();
            text_center(
                &layer,
                "All amounts in Kenyan Shillings (KSh)",
                7.0,
                Mm(ctx.y),
                ctx.font,
                MUTED,
            );
            ctx.y -= 4.0;
        }
        let cols = ctx.two_col_table();
        ctx.draw_table(&cols, &rows, true);
    }

    // ---- SALES ----
    if let Some(ref sales) = stmt.sales {
        let mut rows = vec![
            vec![
                "Sales transactions".into(),
                sales.transaction_count.to_string(),
            ],
            vec!["Gross billed".into(), money_plain(sales.gross_billed)],
            vec!["Cash collected".into(), money_plain(sales.cash_collected)],
            vec![
                "Credit / debt sales (count)".into(),
                sales.debt_transactions.to_string(),
            ],
            vec![
                "Credit / debt sales (billed)".into(),
                money_plain(sales.debt_billed),
            ],
            vec![
                "Product sales".into(),
                sales.product_sales_count.to_string(),
            ],
            vec![
                "Sticker / stock sales".into(),
                sales.stock_sales_count.to_string(),
            ],
        ];
        for pm in &sales.payment_methods {
            rows.push(vec![
                format!("Payment — {}", method_label(&pm.method)),
                format!("{} ({} txn)", money_plain(pm.amount), pm.count),
            ]);
        }
        rows.push(vec![
            "SALES CASH COLLECTED".into(),
            money_plain(sales.cash_collected),
        ]);
        ctx.section_title("SALES SUMMARY", PdfCtx::table_height(rows.len()));
        let cols = ctx.two_col_table();
        ctx.draw_table(&cols, &rows, true);
    }

    // ---- PRINTING ----
    if let Some(ref printing) = stmt.printing {
        let mut rows = vec![
            vec!["Printing jobs".into(), printing.job_count.to_string()],
            vec!["Gross billed".into(), money_plain(printing.gross_billed)],
            vec![
                "Cash collected".into(),
                money_plain(printing.cash_collected),
            ],
            vec![
                "Credit / debt jobs (count)".into(),
                printing.debt_jobs.to_string(),
            ],
            vec![
                "Credit / debt jobs (billed)".into(),
                money_plain(printing.debt_billed),
            ],
            vec![
                "Material used (metres)".into(),
                format!("{:.1}", printing.material_metres_used),
            ],
        ];
        for pm in &printing.payment_methods {
            rows.push(vec![
                format!("Payment — {}", method_label(&pm.method)),
                format!("{} ({} jobs)", money_plain(pm.amount), pm.count),
            ]);
        }
        rows.push(vec![
            "PRINTING CASH COLLECTED".into(),
            money_plain(printing.cash_collected),
        ]);
        ctx.section_title(
            "PRINTING SERVICES SUMMARY",
            PdfCtx::table_height(rows.len()),
        );
        let cols = ctx.two_col_table();
        ctx.draw_table(&cols, &rows, true);
    }

    // ---- MONTHLY ----
    if !stmt.monthly.is_empty() {
        // rows: months + TOTAL
        let month_rows = stmt.monthly.len() + 1;
        ctx.section_title("MONTHLY BREAKDOWN", PdfCtx::table_height(month_rows));
        let show_sales = stmt.sales.is_some();
        let show_print = stmt.printing.is_some();
        let tw = ctx.right - ctx.left;

        let cols: Vec<TableCol> = if show_sales && show_print {
            let c0 = tw * 0.22;
            let c1 = tw * 0.22;
            let c2 = tw * 0.22;
            let c3 = tw * 0.22;
            let c4 = tw * 0.12;
            vec![
                TableCol {
                    title: "Month",
                    x: ctx.left,
                    w: c0,
                    right: false,
                },
                TableCol {
                    title: "Sales",
                    x: ctx.left + c0,
                    w: c1,
                    right: true,
                },
                TableCol {
                    title: "Printing",
                    x: ctx.left + c0 + c1,
                    w: c2,
                    right: true,
                },
                TableCol {
                    title: "Total cash",
                    x: ctx.left + c0 + c1 + c2,
                    w: c3,
                    right: true,
                },
                TableCol {
                    title: "Txns",
                    x: ctx.left + c0 + c1 + c2 + c3,
                    w: c4,
                    right: true,
                },
            ]
        } else if show_sales {
            let c0 = tw * 0.34;
            let c1 = tw * 0.42;
            let c2 = tw * 0.24;
            vec![
                TableCol {
                    title: "Month",
                    x: ctx.left,
                    w: c0,
                    right: false,
                },
                TableCol {
                    title: "Sales cash",
                    x: ctx.left + c0,
                    w: c1,
                    right: true,
                },
                TableCol {
                    title: "Txns",
                    x: ctx.left + c0 + c1,
                    w: c2,
                    right: true,
                },
            ]
        } else {
            let c0 = tw * 0.34;
            let c1 = tw * 0.42;
            let c2 = tw * 0.24;
            vec![
                TableCol {
                    title: "Month",
                    x: ctx.left,
                    w: c0,
                    right: false,
                },
                TableCol {
                    title: "Printing cash",
                    x: ctx.left + c0,
                    w: c1,
                    right: true,
                },
                TableCol {
                    title: "Jobs",
                    x: ctx.left + c0 + c1,
                    w: c2,
                    right: true,
                },
            ]
        };

        let mut rows: Vec<Vec<String>> = stmt
            .monthly
            .iter()
            .map(|m| {
                if show_sales && show_print {
                    vec![
                        m.label.clone(),
                        money_plain(m.sales_revenue),
                        money_plain(m.printing_revenue),
                        money_plain(m.total_revenue),
                        m.total_count.to_string(),
                    ]
                } else if show_sales {
                    vec![
                        m.label.clone(),
                        money_plain(m.sales_revenue),
                        m.sales_count.to_string(),
                    ]
                } else {
                    vec![
                        m.label.clone(),
                        money_plain(m.printing_revenue),
                        m.printing_count.to_string(),
                    ]
                }
            })
            .collect();

        let sum_sales: f64 = stmt.monthly.iter().map(|m| m.sales_revenue).sum();
        let sum_print: f64 = stmt.monthly.iter().map(|m| m.printing_revenue).sum();
        let sum_total: f64 = stmt.monthly.iter().map(|m| m.total_revenue).sum();
        let sum_cnt: i64 = stmt.monthly.iter().map(|m| m.total_count).sum();
        if show_sales && show_print {
            rows.push(vec![
                "TOTAL".into(),
                money_plain(sum_sales),
                money_plain(sum_print),
                money_plain(sum_total),
                sum_cnt.to_string(),
            ]);
        } else if show_sales {
            rows.push(vec![
                "TOTAL".into(),
                money_plain(sum_sales),
                sum_cnt.to_string(),
            ]);
        } else {
            rows.push(vec![
                "TOTAL".into(),
                money_plain(sum_print),
                sum_cnt.to_string(),
            ]);
        }

        ctx.draw_table(&cols, &rows, true);
    }

    // ---- Disclaimer ----
    {
        let disclaimer = "Disclaimer: This statement is generated from MULTIPRINTS business records and is intended as supporting evidence of trading activity for business and financing purposes. It is not a formal audited financial statement. Cash collected includes paid (non-credit) sales/jobs plus debt repayments received in the period. Figures are in Kenyan Shillings (KSh).";
        let max_w = ctx.right - ctx.left;
        let mut lines: Vec<String> = Vec::new();
        let mut line = String::new();
        for word in disclaimer.split_whitespace() {
            let trial = if line.is_empty() {
                word.to_string()
            } else {
                format!("{line} {word}")
            };
            if text_width_mm(&trial, 6.3) > max_w {
                lines.push(line);
                line = word.to_string();
            } else {
                line = trial;
            }
        }
        if !line.is_empty() {
            lines.push(line);
        }
        let block_h = 3.2 * lines.len() as f32 + 2.0;
        ctx.ensure(block_h);
        let layer = ctx.layer();
        for l in &lines {
            text_at(&layer, l, 6.3, Mm(ctx.left), Mm(ctx.y), ctx.font, MUTED);
            ctx.y -= 3.2;
        }
    }

    // Stamp footers / page numbers on every page
    let total = ctx.page_no();
    for i in 0..total {
        ctx.draw_footer_and_page_no(i, total);
    }

    let mut buf = BufWriter::new(Vec::new());
    doc.save(&mut buf).map_err(|e| format!("PDF save: {e}"))?;
    buf.into_inner().map_err(|e| format!("PDF buffer: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::*;

    fn sample_statement() -> BusinessStatement {
        BusinessStatement {
            source: "both".into(),
            months: 3,
            period_start: "2026-04-22".into(),
            period_end: "2026-07-22".into(),
            generated_at: "2026-07-22 15:30".into(),
            requested_by: Some("admin".into()),
            app_version: env!("CARGO_PKG_VERSION").into(),
            total_gross_billed: 485_750.0,
            total_cash_collected: 412_300.0,
            total_transactions: 187,
            average_monthly_cash: 137_433.0,
            period_outstanding_receivables: 48_200.0,
            sales: Some(StatementSalesSection {
                transaction_count: 124,
                gross_billed: 298_500.0,
                cash_collected: 261_800.0,
                debt_transactions: 11,
                debt_billed: 36_700.0,
                product_sales_count: 86,
                stock_sales_count: 38,
                payment_methods: vec![
                    StatementPaymentBreakdown {
                        method: "mpesa".into(),
                        amount: 148_200.0,
                        count: 67,
                    },
                    StatementPaymentBreakdown {
                        method: "cash".into(),
                        amount: 92_400.0,
                        count: 41,
                    },
                    StatementPaymentBreakdown {
                        method: "till".into(),
                        amount: 21_200.0,
                        count: 5,
                    },
                ],
            }),
            printing: Some(StatementPrintingSection {
                job_count: 63,
                gross_billed: 187_250.0,
                cash_collected: 150_500.0,
                debt_jobs: 8,
                debt_billed: 36_750.0,
                material_metres_used: 214.5,
                payment_methods: vec![
                    StatementPaymentBreakdown {
                        method: "mpesa".into(),
                        amount: 98_000.0,
                        count: 39,
                    },
                    StatementPaymentBreakdown {
                        method: "cash".into(),
                        amount: 52_500.0,
                        count: 24,
                    },
                ],
            }),
            monthly: vec![
                StatementMonthRow {
                    year_month: "2026-04".into(),
                    label: "Apr 2026".into(),
                    sales_revenue: 48_200.0,
                    sales_count: 22,
                    printing_revenue: 31_000.0,
                    printing_count: 12,
                    total_revenue: 79_200.0,
                    total_count: 34,
                },
                StatementMonthRow {
                    year_month: "2026-05".into(),
                    label: "May 2026".into(),
                    sales_revenue: 72_100.0,
                    sales_count: 35,
                    printing_revenue: 44_800.0,
                    printing_count: 18,
                    total_revenue: 116_900.0,
                    total_count: 53,
                },
                StatementMonthRow {
                    year_month: "2026-06".into(),
                    label: "Jun 2026".into(),
                    sales_revenue: 81_500.0,
                    sales_count: 38,
                    printing_revenue: 39_200.0,
                    printing_count: 16,
                    total_revenue: 120_700.0,
                    total_count: 54,
                },
                StatementMonthRow {
                    year_month: "2026-07".into(),
                    label: "Jul 2026".into(),
                    sales_revenue: 60_000.0,
                    sales_count: 29,
                    printing_revenue: 35_500.0,
                    printing_count: 17,
                    total_revenue: 95_500.0,
                    total_count: 46,
                },
            ],
        }
    }

    #[test]
    fn renders_empty_both_statement() {
        let bytes = render_business_statement_pdf(&sample_statement()).expect("pdf");
        assert!(bytes.starts_with(b"%PDF"), "should be a PDF");
        assert!(bytes.len() > 500);
    }

    #[test]
    fn write_sample_statement_pdf() {
        let bytes = render_business_statement_pdf(&sample_statement()).expect("pdf");
        let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("sample-business-statement.pdf");
        std::fs::write(&out, &bytes).expect("write pdf");
        eprintln!(
            "Wrote sample PDF to {}",
            out.canonicalize().unwrap_or(out).display()
        );
        assert!(bytes.starts_with(b"%PDF"));
    }

    #[test]
    fn money_formats_with_commas() {
        assert_eq!(money_plain(412_300.0), "412,300");
        assert_eq!(money_plain(0.0), "0");
        assert_eq!(money_plain(1_234.0), "1,234");
    }
}
