// Real, scannable QR rendering of the backend's signed token.
import QRCode from 'qrcode';

export async function qrDataUrl(text: string, size = 180): Promise<string> {
  return QRCode.toDataURL(text, {
    errorCorrectionLevel: 'M',
    margin: 1,
    width: size,
    color: { dark: '#141a16', light: '#ffffff' }
  });
}
