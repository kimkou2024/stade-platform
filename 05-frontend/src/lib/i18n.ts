// Trilingual store [M §13]. Arabic primary + RTL; French & English.
import { writable, derived } from 'svelte/store';

export type Locale = 'ar' | 'fr' | 'en';
export const RTL: Record<Locale, boolean> = { ar: true, fr: false, en: false };

type Dict = Record<string, string>;
const dict: Record<Locale, Dict> = {
  fr: {
    app: 'Stade Ali Ammar', next: 'Prochain match', buy: 'Acheter des billets', upcoming: 'Événements à venir',
    back: 'Retour', choose: 'Choisir une zone', qty: 'Nombre de billets', qtyhint: 'Maximum 5 billets par compte.',
    checkout: 'Paiement', total: 'Total', hold: 'Places réservées —', paym: 'Moyen de paiement',
    secure: "Passerelle agréée Banque d'Algérie. Aucune donnée carte stockée. 2FA requis.", loi: 'Données traitées conformément à la Loi 18-07.',
    paid: 'Paiement accepté', holder: 'Titulaire', zone: 'Zone', gate: 'Porte', done: 'Terminer', continue: 'Continuer', pay: 'Payer',
    login: 'Connexion', available: 'disponible', offline: 'QR vérifié hors ligne aux portiques (clé publique).'
  },
  en: {
    app: 'Ali Ammar Stadium', next: 'Next match', buy: 'Buy tickets', upcoming: 'Upcoming events',
    back: 'Back', choose: 'Choose a zone', qty: 'Number of tickets', qtyhint: 'Maximum 5 tickets per account.',
    checkout: 'Checkout', total: 'Total', hold: 'Seats held —', paym: 'Payment method',
    secure: 'Bank-of-Algeria approved gateway. No card data stored. 2FA required.', loi: 'Data processed under Law 18-07.',
    paid: 'Payment accepted', holder: 'Holder', zone: 'Zone', gate: 'Gate', done: 'Done', continue: 'Continue', pay: 'Pay',
    login: 'Login', available: 'available', offline: 'QR verified offline at the gates (public key).'
  },
  ar: {
    app: 'ملعب علي عمار', next: 'المباراة القادمة', buy: 'شراء التذاكر', upcoming: 'الفعاليات القادمة',
    back: 'رجوع', choose: 'اختر منطقة', qty: 'عدد التذاكر', qtyhint: 'بحد أقصى 5 تذاكر لكل حساب.',
    checkout: 'الدفع', total: 'المجموع', hold: 'تم حجز المقاعد —', paym: 'وسيلة الدفع',
    secure: 'بوابة معتمدة من بنك الجزائر. لا تُخزَّن بيانات البطاقة. يلزم التحقق بخطوتين.', loi: 'تُعالَج البيانات وفق القانون 18-07.',
    paid: 'تم قبول الدفع', holder: 'حامل التذكرة', zone: 'المنطقة', gate: 'البوابة', done: 'إنهاء', continue: 'متابعة', pay: 'ادفع',
    login: 'تسجيل الدخول', available: 'متاح', offline: 'يُتحقَّق من الرمز دون اتصال عند البوابات (المفتاح العام).'
  }
};

export const locale = writable<Locale>('fr');
export const t = derived(locale, ($l) => (key: string) => dict[$l][key] ?? key);
export function fmtDZD(n: number, l: Locale) {
  return n.toLocaleString(l === 'ar' ? 'ar-DZ' : 'fr-DZ') + ' DZD';
}
