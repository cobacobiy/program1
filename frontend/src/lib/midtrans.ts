declare global {
  interface Window {
    snap?: {
      pay: (
        token: string,
        options: {
          onSuccess?: (result: any) => void;
          onPending?: (result: any) => void;
          onError?: (result: any) => void;
          onClose?: () => void;
        }
      ) => void;
    };
  }
}

export function loadMidtransSnap(clientKey: string, isProduction = false): Promise<void> {
  return new Promise((resolve, reject) => {
    if (typeof window === 'undefined') return resolve();
    if (window.snap) return resolve();

    const scriptUrl = isProduction
      ? 'https://app.midtrans.com/snap/snap.js'
      : 'https://app.sandbox.midtrans.com/snap/snap.js';

    const script = document.createElement('script');
    script.src = scriptUrl;
    script.setAttribute('data-client-key', clientKey);
    script.async = true;

    script.onload = () => resolve();
    script.onerror = () => reject(new Error('Gagal memuat script Midtrans Snap SDK'));

    document.body.appendChild(script);
  });
}
