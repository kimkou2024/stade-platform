import { json } from '@sveltejs/kit';

/**
 * Digital Asset Links for the StadeDz Billets TWA (Trusted Web Activity).
 * Lets the Android app open this site full-screen without a browser URL bar.
 *
 * Fingerprints:
 *  - EE:9A:… = debug keystore  (testing the debug APK)
 *  - D4:5C:… = fan upload key  (direct-signed AAB / sideload)
 *  - After the first Play upload, add Google's **App signing key** SHA-256
 *    (Play Console → Test and release → Setup → App signing) to this list,
 *    otherwise the published app will show a URL bar.
 */
const FINGERPRINTS = [
  // Google Play App Signing key (signs the PUBLISHED app) — the important one
  '0A:5D:8A:11:52:07:E9:11:79:FA:C8:11:91:EE:7E:0C:BB:D5:71:A5:2B:02:F7:65:37:CB:A1:D5:4D:8E:6D:14',
  'EE:9A:F5:86:99:CD:6E:AA:14:84:D8:96:14:8A:E3:D9:9C:3B:EB:D9:31:D1:90:2F:54:C7:24:DC:9B:04:DC:87', // debug keystore (testing)
  'D4:5C:1D:FD:D6:D4:B9:94:C5:8C:60:72:DD:0B:1E:5A:44:EB:09:F6:99:D5:00:7B:73:29:60:31:DA:72:31:86'  // fan upload key
];

export const prerender = false;

export function GET() {
  return json([
    {
      relation: ['delegate_permission/common.handle_all_urls'],
      target: {
        namespace: 'android_app',
        package_name: 'dz.sogisl.stade.fan',
        sha256_cert_fingerprints: FINGERPRINTS
      }
    }
  ]);
}
