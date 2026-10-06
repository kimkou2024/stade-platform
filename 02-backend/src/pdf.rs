//! PDF generation [M §7.1.5, §9.2]: spectator list and cashier ticket (with a
//! real, scannable QR drawn as vector modules — no raster image needed).

use printpdf::*;
use qrcode::{Color as QrColor, QrCode};

/// A4 spectator/ticket list as a simple paginated PDF [M §9.2].
pub fn spectator_list(title: &str, rows: &[(String, String, String)]) -> Vec<u8> {
    let (doc, page, layer) = PdfDocument::new(title, Mm(210.0), Mm(297.0), "Layer 1");
    let font = doc.add_builtin_font(BuiltinFont::Helvetica).unwrap();
    let bold = doc.add_builtin_font(BuiltinFont::HelveticaBold).unwrap();
    let mut cur = doc.get_page(page).get_layer(layer);
    cur.use_text(title, 16.0, Mm(15.0), Mm(282.0), &bold);
    cur.use_text("Titulaire / Zone / Statut", 10.0, Mm(15.0), Mm(274.0), &font);
    let mut y: f32 = 266.0;
    for (holder, zone, status) in rows {
        if y < 15.0 {
            let (p, l) = doc.add_page(Mm(210.0), Mm(297.0), "Layer");
            cur = doc.get_page(p).get_layer(l);
            y = 282.0;
        }
        cur.use_text(format!("{holder}  -  {zone}  -  {status}"), 10.0, Mm(15.0), Mm(y), &font);
        y -= 6.0;
    }
    let mut buf = Vec::new();
    doc.save(&mut std::io::BufWriter::new(&mut buf)).ok();
    buf
}

/// Cashier e-ticket as a PDF with a scannable QR of the signed token [M §7.1.5].
pub fn cashier_ticket(event: &str, holder: &str, zone: &str, ticket_id: &str, qr_token: &str) -> Vec<u8> {
    let (doc, page, layer) = PdfDocument::new("Billet", Mm(210.0), Mm(297.0), "Layer 1");
    let font = doc.add_builtin_font(BuiltinFont::Helvetica).unwrap();
    let bold = doc.add_builtin_font(BuiltinFont::HelveticaBold).unwrap();
    let cur = doc.get_page(page).get_layer(layer);
    cur.use_text("Stade Ali Ammar — Billet electronique", 16.0, Mm(20.0), Mm(270.0), &bold);
    cur.use_text(event, 13.0, Mm(20.0), Mm(260.0), &font);
    cur.use_text(format!("Titulaire : {holder}"), 11.0, Mm(20.0), Mm(250.0), &font);
    cur.use_text(format!("Zone : {zone}"), 11.0, Mm(20.0), Mm(243.0), &font);
    cur.use_text(format!("ID : {ticket_id}"), 9.0, Mm(20.0), Mm(236.0), &font);
    cur.use_text("Signature Ed25519 — verifiable hors ligne aux portiques.", 8.0, Mm(20.0), Mm(150.0), &font);

    // Draw the QR as filled vector squares (scannable).
    if let Ok(code) = QrCode::new(qr_token.as_bytes()) {
        let n = code.width();
        let colors = code.to_colors();
        let module: f32 = 2.2; // mm per module
        let x0: f32 = 20.0;
        let y_top: f32 = 225.0; // top of the QR block (PDF y grows upward)
        cur.set_fill_color(Color::Greyscale(Greyscale::new(0.0, None)));
        for row in 0..n {
            for col in 0..n {
                if colors[row * n + col] == QrColor::Dark {
                    let x = x0 + col as f32 * module;
                    let y = y_top - row as f32 * module;
                    let ring = vec![
                        (Point::new(Mm(x), Mm(y)), false),
                        (Point::new(Mm(x + module), Mm(y)), false),
                        (Point::new(Mm(x + module), Mm(y - module)), false),
                        (Point::new(Mm(x), Mm(y - module)), false),
                    ];
                    cur.add_polygon(Polygon {
                        rings: vec![ring],
                        mode: PolygonMode::Fill,
                        winding_order: WindingOrder::NonZero,
                    });
                }
            }
        }
    }
    let mut buf = Vec::new();
    doc.save(&mut std::io::BufWriter::new(&mut buf)).ok();
    buf
}
