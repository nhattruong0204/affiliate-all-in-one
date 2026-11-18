/*
 * Progressive enhancement for the UI shell (no framework, no eval, no inline handlers: CSP-safe).
 * Everything here has a server-rendered baseline; without JS pages still render and navigate.
 *
 *  - Theme toggle: sets the `aao_theme` cookie + <html data-theme>. The server reads the cookie, so
 *    the next page load is correct before first paint.
 *  - Language links: remember the choice in `aao_lang` (the server sends `/` to `/en` when it is `en`).
 *  - Toasts: window.AAO.toast({ kind, message, title?, timeout? }); server-rendered toasts get
 *    the same dismiss behaviour.
 *  - Submit guard: forms marked data-submit-guard disable double submission.
 */
(() => {
  'use strict';

  const ONE_YEAR = 60 * 60 * 24 * 365;
  const root = document.documentElement;

  const setCookie = (name, value) => {
    const secure = location.protocol === 'https:' ? '; Secure' : '';
    document.cookie = `${name}=${value}; Path=/; Max-Age=${ONE_YEAR}; SameSite=Lax${secure}`;
  };

  /* ---- Theme ---------------------------------------------------------------------------- */
  const prefersDark = window.matchMedia('(prefers-color-scheme: dark)');
  const currentTheme = () => {
    const explicit = root.getAttribute('data-theme');
    if (explicit === 'light' || explicit === 'dark') return explicit;
    return prefersDark.matches ? 'dark' : 'light';
  };
  const syncToggle = () => {
    const pressed = currentTheme() === 'dark' ? 'true' : 'false';
    document.querySelectorAll('[data-theme-toggle]').forEach((el) => el.setAttribute('aria-pressed', pressed));
  };
  prefersDark.addEventListener('change', syncToggle);
  syncToggle();

  /* ---- Toasts --------------------------------------------------------------------------- */
  const dismiss = (toast) => {
    if (!toast || toast.hasAttribute('data-leaving')) return;
    toast.setAttribute('data-leaving', '');
    const remove = () => toast.remove();
    toast.addEventListener('animationend', remove, { once: true });
    window.setTimeout(remove, 400); // fallback when animations are disabled
  };

  const armAutoDismiss = (toast, timeout) => {
    if (!timeout) return;
    let timer = 0;
    const start = () => { timer = window.setTimeout(() => dismiss(toast), timeout); };
    const stop = () => window.clearTimeout(timer);
    toast.addEventListener('mouseenter', stop);
    toast.addEventListener('focusin', stop);
    toast.addEventListener('mouseleave', start);
    toast.addEventListener('focusout', start);
    start();
  };

  const KINDS = ['info', 'success', 'warning', 'error'];
  const toast = ({ kind = 'info', message = '', title, timeout } = {}) => {
    const region = document.querySelector('[data-toast-region]');
    const template = region && region.querySelector('template[data-toast-template]');
    if (!template) return null;
    const safeKind = KINDS.includes(kind) ? kind : 'info';
    const node = template.content.firstElementChild.cloneNode(true);
    node.setAttribute('data-kind', safeKind);
    node.setAttribute('role', safeKind === 'error' ? 'alert' : 'status');
    const titles = region.dataset;
    const heading = title || titles[`title${safeKind[0].toUpperCase()}${safeKind.slice(1)}`] || '';
    node.querySelector('[data-toast-title]').textContent = heading;
    node.querySelector('[data-toast-message]').textContent = message;
    region.appendChild(node);
    // Errors stay until dismissed (WCAG 2.2.1: no hidden time limit on important messages).
    armAutoDismiss(node, timeout ?? (safeKind === 'error' ? 0 : 6000));
    return node;
  };
  window.AAO = Object.freeze({ toast });

  document.querySelectorAll('[data-toast][data-timeout]').forEach((el) => {
    armAutoDismiss(el, Number.parseInt(el.getAttribute('data-timeout'), 10) || 0);
  });

  /* ---- Delegated events ----------------------------------------------------------------- */
  document.addEventListener('click', (event) => {
    const target = event.target instanceof Element ? event.target : null;
    if (!target) return;

    const themeButton = target.closest('[data-theme-toggle]');
    if (themeButton) {
      const next = currentTheme() === 'dark' ? 'light' : 'dark';
      root.setAttribute('data-theme', next);
      setCookie('aao_theme', next);
      syncToggle();
      return;
    }

    const langLink = target.closest('[data-lang-link]');
    if (langLink) {
      setCookie('aao_lang', langLink.getAttribute('data-lang-link'));
      return;
    }

    const close = target.closest('[data-toast-close]');
    if (close) {
      dismiss(close.closest('[data-toast]'));
      return;
    }

    const demo = target.closest('[data-toast-demo]');
    if (demo) {
      toast({ kind: demo.getAttribute('data-toast-demo'), message: demo.getAttribute('data-demo-message') || '' });
    }
  });

  document.addEventListener('keydown', (event) => {
    if (event.key !== 'Escape') return;
    const focused = document.activeElement instanceof Element ? document.activeElement.closest('[data-toast]') : null;
    if (focused) dismiss(focused);
  });

  document.addEventListener('submit', (event) => {
    const form = event.target;
    if (!(form instanceof HTMLFormElement) || !form.hasAttribute('data-submit-guard')) return;
    if (form.hasAttribute('data-submitting')) {
      event.preventDefault();
      return;
    }
    form.setAttribute('data-submitting', '');
    form.querySelectorAll('button[type="submit"]').forEach((b) => b.setAttribute('aria-busy', 'true'));
  });

  // Restored from the back/forward cache: the form is usable again.
  window.addEventListener('pageshow', (event) => {
    if (!event.persisted) return;
    document.querySelectorAll('form[data-submitting]').forEach((form) => {
      form.removeAttribute('data-submitting');
      form.querySelectorAll('button[aria-busy]').forEach((b) => b.removeAttribute('aria-busy'));
    });
  });
})();
