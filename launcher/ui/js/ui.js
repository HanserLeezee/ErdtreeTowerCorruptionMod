/* ============================================================
   通用 UI 原语层

   官方对应的模块：
     .hyp-toast-container   轻提示
     .hyp-el-tooltip        悬浮提示
     .hyp-account-popup     账号弹窗（这里做成通用浮层）
     .hyp-el-skeleton       骨架屏
   另外提供：i18n、焦点陷阱、动效偏好。

   所有原语都挂在 window.hoyoUI 上，app.js 按需调用。
   ============================================================ */

(function () {
  'use strict';

  /* ---------------- i18n ---------------- */
  var locale = 'zh-CN';
  var dicts = {};

  function setLocale(l, d) {
    locale = l || locale;
    if (d) dicts[locale] = d;
    document.documentElement.setAttribute('lang', locale === 'zh-CN' ? 'zh-CN' : locale);
  }

  function t(key, fallback) {
    var d = dicts[locale] || {};
    if (key in d) return d[key];
    var zh = dicts['zh-CN'] || {};
    if (key in zh) return zh[key];
    return fallback != null ? fallback : key;
  }

  function locales() { return Object.keys(dicts); }

  /* ---------------- 焦点圈 ---------------- */
  /* 只在键盘操作时显示焦点轮廓，鼠标点击不显示 */
  function initFocusRing() {
    var root = document.documentElement;
    document.addEventListener('keydown', function (e) {
      if (e.key === 'Tab') root.classList.add('kb');
    });
    document.addEventListener('mousedown', function () {
      root.classList.remove('kb');
    });
  }

  /* ---------------- 焦点陷阱 ---------------- */
  var trapStack = [];

  function focusables(box) {
    return Array.prototype.slice.call(box.querySelectorAll(
      'a[href], button:not([disabled]), input:not([disabled]), select, textarea,' +
      '[tabindex]:not([tabindex="-1"])'
    )).filter(function (e) { return e.offsetParent !== null || e === document.activeElement; });
  }

  function trapFocus(box) {
    var prev = document.activeElement;
    var entry = {
      box: box, prev: prev,
      onKey: function (e) {
        if (e.key !== 'Tab') return;
        var f = focusables(box);
        if (!f.length) return;
        var first = f[0], last = f[f.length - 1];
        if (e.shiftKey && document.activeElement === first) {
          e.preventDefault(); last.focus();
        } else if (!e.shiftKey && document.activeElement === last) {
          e.preventDefault(); first.focus();
        }
      },
    };
    document.addEventListener('keydown', entry.onKey, true);
    trapStack.push(entry);
    var f = focusables(box);
    if (f.length) {
      /* 优先聚焦第一个非关闭按钮，避免一打开焦点就在 × 上 */
      (f.length > 1 ? f[1] : f[0]).focus();
    }
    return function release() {
      document.removeEventListener('keydown', entry.onKey, true);
      var i = trapStack.indexOf(entry);
      if (i >= 0) trapStack.splice(i, 1);
      if (prev && prev.focus) try { prev.focus(); } catch (err) { /* ignore */ }
    };
  }

  /* ---------------- Toast ---------------- */
  var toastHost = null;

  function ensureToastHost() {
    if (toastHost && document.body.contains(toastHost)) return toastHost;
    toastHost = document.createElement('div');
    /* 官方是 #hyp-toast-container */
    toastHost.id = 'hyp-toast-container';
    toastHost.className = 'toast-host';
    document.body.appendChild(toastHost);
    return toastHost;
  }

  var ICONS = {
    info: '<svg viewBox="0 0 16 16"><circle cx="8" cy="8" r="7" fill="none" ' +
      'stroke="currentColor" stroke-width="1.4"/><path d="M8 7v4.2M8 4.4v1.4" ' +
      'stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/></svg>',
    success: '<svg viewBox="0 0 16 16"><circle cx="8" cy="8" r="7" fill="none" ' +
      'stroke="currentColor" stroke-width="1.4"/><path d="M4.8 8.2l2.2 2.2 4.2-4.6" ' +
      'fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" ' +
      'stroke-linejoin="round"/></svg>',
    warn: '<svg viewBox="0 0 16 16"><path d="M8 1.8l6.4 11.4H1.6z" fill="none" ' +
      'stroke="currentColor" stroke-width="1.4" stroke-linejoin="round"/>' +
      '<path d="M8 6v3.2M8 11.2v1.2" stroke="currentColor" stroke-width="1.6" ' +
      'stroke-linecap="round"/></svg>',
    error: '<svg viewBox="0 0 16 16"><circle cx="8" cy="8" r="7" fill="none" ' +
      'stroke="currentColor" stroke-width="1.4"/><path d="M5.4 5.4l5.2 5.2M10.6 5.4l-5.2 5.2" ' +
      'stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/></svg>',
  };

  /* 同一时刻只留少量提示，避免刷屏 */
  var MAX_TOAST = 3;

  function toast(msg, opts) {
    opts = opts || {};
    var host = ensureToastHost();
    while (host.children.length >= MAX_TOAST) host.removeChild(host.firstChild);

    var type = opts.type || 'info';
    var box = document.createElement('div');
    box.className = 'toast toast--' + type;
    box.setAttribute('role', 'status');
    box.innerHTML = '<i class="toast__icon">' + (ICONS[type] || ICONS.info) + '</i>' +
      '<span class="toast__msg"></span>' +
      (opts.action ? '<button class="toast__act"></button>' : '');
    box.querySelector('.toast__msg').textContent = msg;
    if (opts.action) {
      var b = box.querySelector('.toast__act');
      b.textContent = opts.action;
      b.addEventListener('click', function () {
        if (opts.onAction) opts.onAction();
        hide();
      });
    }
    host.appendChild(box);

    /* 进入动画 */
    requestAnimationFrame(function () { box.classList.add('in'); });

    var timer = null;
    function hide() {
      if (timer) { clearTimeout(timer); timer = null; }
      box.classList.remove('in');
      box.classList.add('out');
      setTimeout(function () { if (box.parentNode) box.parentNode.removeChild(box); }, 220);
    }
    var dur = opts.duration == null ? 2600 : opts.duration;
    if (dur > 0) timer = setTimeout(hide, dur);
    box.addEventListener('click', function (e) {
      if (e.target === b) return;
      hide();
    });
    return { close: hide, el: box };
  }

  /* ---------------- Tooltip ---------------- */
  var tipEl = null;

  function ensureTip() {
    if (tipEl && document.body.contains(tipEl)) return tipEl;
    tipEl = document.createElement('div');
    tipEl.className = 'tooltip';
    tipEl.setAttribute('role', 'tooltip');
    document.body.appendChild(tipEl);
    return tipEl;
  }

  function showTip(target) {
    var text = target.getAttribute('data-tip');
    if (!text) return;
    var el = ensureTip();
    el.textContent = text;
    el.classList.add('on');
    var r = target.getBoundingClientRect();
    var tr = el.getBoundingClientRect();
    var place = target.getAttribute('data-tip-place') || 'left';
    var x, y;
    if (place === 'left') {
      x = r.left - tr.width - 10;
      y = r.top + r.height / 2 - tr.height / 2;
    } else if (place === 'right') {
      x = r.right + 10;
      y = r.top + r.height / 2 - tr.height / 2;
    } else if (place === 'top') {
      x = r.left + r.width / 2 - tr.width / 2;
      y = r.top - tr.height - 8;
    } else {
      x = r.left + r.width / 2 - tr.width / 2;
      y = r.bottom + 8;
    }
    /* 不出画布 */
    x = Math.max(6, Math.min(x, innerWidth - tr.width - 6));
    y = Math.max(6, Math.min(y, innerHeight - tr.height - 6));
    el.style.left = Math.round(x) + 'px';
    el.style.top = Math.round(y) + 'px';
  }

  function hideTip() {
    if (tipEl) tipEl.classList.remove('on');
  }

  function initTooltips(root) {
    var host = root || document;
    var timer = null;
    host.addEventListener('mouseover', function (e) {
      var target = e.target.closest('[data-tip]');
      if (!target || target === (tipEl && tipEl._for)) return;
      clearTimeout(timer);
      timer = setTimeout(function () {
        if (tipEl) tipEl._for = target;
        showTip(target);
      }, 240);   /* 官方 tooltip 也有延迟，避免扫过就弹 */
    });
    host.addEventListener('mouseout', function (e) {
      if (!e.target.closest('[data-tip]')) return;
      clearTimeout(timer);
      hideTip();
    });
    host.addEventListener('mousedown', function () {
      clearTimeout(timer);
      hideTip();
    });
    window.addEventListener('scroll', hideTip, true);
  }

  /* ---------------- 浮层（账号弹窗 / 上下文菜单共用） ---------------- */
  var popupEl = null;

  function closePopup() {
    if (popupEl && popupEl.parentNode) popupEl.parentNode.removeChild(popupEl);
    popupEl = null;
  }

  function openPopup(anchor, html, opts) {
    opts = opts || {};
    closePopup();
    var el = document.createElement('div');
    el.className = 'popup ' + (opts.cls || '');
    el.setAttribute('role', 'dialog');
    el.innerHTML = html;
    document.body.appendChild(el);
    popupEl = el;

    var r = anchor.getBoundingClientRect();
    var pr = el.getBoundingClientRect();
    var x = opts.align === 'left' ? r.left : r.right - pr.width;
    var y = r.bottom + 8;
    x = Math.max(6, Math.min(x, innerWidth - pr.width - 6));
    if (y + pr.height > innerHeight - 6) y = r.top - pr.height - 8;
    el.style.left = Math.round(x) + 'px';
    el.style.top = Math.round(Math.max(6, y)) + 'px';

    requestAnimationFrame(function () { el.classList.add('in'); });

    function onDoc(e) {
      if (el.contains(e.target) || anchor.contains(e.target)) return;
      closePopup();
    }
    setTimeout(function () { document.addEventListener('mousedown', onDoc); }, 0);
    el._cleanup = function () { document.removeEventListener('mousedown', onDoc); };
    return el;
  }

  var _closePopup = closePopup;
  closePopup = function () {
    if (popupEl && popupEl._cleanup) popupEl._cleanup();
    _closePopup();
  };

  /* ---------------- 骨架屏 ---------------- */
  function skeleton(lines, cls) {
    var n = lines || 3;
    var out = '<div class="sk ' + (cls || '') + '">';
    for (var i = 0; i < n; i++) {
      var w = 100 - (i % 3) * 12;
      out += '<i style="width:' + w + '%"></i>';
    }
    return out + '</div>';
  }

  /* ---------------- 动效偏好 ---------------- */
  function applyMotion(reduced) {
    document.documentElement.classList.toggle('reduced-motion', !!reduced);
  }

  initFocusRing();

  window.hoyoUI = {
    t: t, setLocale: setLocale, locales: locales,
    toast: toast,
    initTooltips: initTooltips,
    openPopup: openPopup, closePopup: closePopup,
    trapFocus: trapFocus,
    skeleton: skeleton,
    applyMotion: applyMotion,
  };
})();
