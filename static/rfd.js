/**
 * OpenRFD — Client-side JavaScript
 *
 * Features:
 * - Annotation sidebar rendering and highlight linking
 * - Text selection for annotation creation
 * - GitHub OAuth (device flow + PKCE)
 * - Live PR discussion (fetched via GitHub API)
 * - Cmd+K search modal
 * - Table sorting and state filtering on index page
 */

(function () {
  'use strict';

  // ---------------------------------------------------------------------------
  // GitHub OAuth
  // ---------------------------------------------------------------------------

  var TOKEN_KEY = 'openrfd_github_token';
  var USER_KEY = 'openrfd_github_user';

  function getToken() { return localStorage.getItem(TOKEN_KEY); }
  function getUser() { try { return JSON.parse(localStorage.getItem(USER_KEY)); } catch (e) { return null; } }
  function setAuth(token, user) { localStorage.setItem(TOKEN_KEY, token); localStorage.setItem(USER_KEY, JSON.stringify(user)); }
  function clearAuth() { localStorage.removeItem(TOKEN_KEY); localStorage.removeItem(USER_KEY); }

  async function deviceFlowLogin(clientId) {
    var codeRes = await fetch('https://github.com/login/device/code', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', 'Accept': 'application/json' },
      body: JSON.stringify({ client_id: clientId, scope: 'repo' })
    });
    var codeData = await codeRes.json();
    var msg = 'Enter code ' + codeData.user_code + ' at github.com/login/device';
    if (!confirm(msg + '\n\nClick OK after entering the code.')) return null;
    window.open(codeData.verification_uri, '_blank');
    var interval = (codeData.interval || 5) * 1000;
    var expires = Date.now() + (codeData.expires_in || 900) * 1000;
    while (Date.now() < expires) {
      await new Promise(function (r) { setTimeout(r, interval); });
      var tokenRes = await fetch('https://github.com/login/oauth/access_token', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', 'Accept': 'application/json' },
        body: JSON.stringify({ client_id: clientId, device_code: codeData.device_code, grant_type: 'urn:ietf:params:oauth:grant-type:device_code' })
      });
      var tokenData = await tokenRes.json();
      if (tokenData.access_token) {
        var userRes = await fetch('https://api.github.com/user', { headers: { Authorization: 'Bearer ' + tokenData.access_token } });
        var user = await userRes.json();
        setAuth(tokenData.access_token, { login: user.login, name: user.name, avatar: user.avatar_url });
        return tokenData.access_token;
      }
      if (tokenData.error === 'expired_token') break;
    }
    return null;
  }

  async function githubFetch(path) {
    var token = getToken();
    if (!token) return null;
    var res = await fetch('https://api.github.com' + path, {
      headers: { Authorization: 'Bearer ' + token, Accept: 'application/vnd.github.v3+json' }
    });
    if (res.status === 401) { clearAuth(); return null; }
    return res.json();
  }

  // ---------------------------------------------------------------------------
  // Annotations
  // ---------------------------------------------------------------------------

  function renderAnnotations(annotations) {
    var list = document.getElementById('annotation-list');
    var count = document.getElementById('annotation-count');
    if (!list || !annotations) return;
    var topLevel = [], replies = {};
    for (var i = 0; i < annotations.length; i++) {
      var a = annotations[i];
      if (a.motivation === 'replying' && typeof a.target === 'string') {
        if (!replies[a.target]) replies[a.target] = [];
        replies[a.target].push(a);
      } else {
        topLevel.push(a);
      }
    }
    if (count) count.textContent = '(' + annotations.length + ')';
    list.innerHTML = '';
    for (var j = 0; j < topLevel.length; j++) {
      var card = createAnnotationCard(topLevel[j]);
      if (replies[topLevel[j].id]) {
        var rd = document.createElement('div');
        rd.className = 'annotation-replies';
        for (var k = 0; k < replies[topLevel[j].id].length; k++) {
          rd.appendChild(createAnnotationCard(replies[topLevel[j].id][k]));
        }
        card.appendChild(rd);
      }
      list.appendChild(card);
    }
    highlightAnnotations(topLevel);
  }

  function createAnnotationCard(a) {
    var card = document.createElement('div');
    card.className = 'annotation-card' + (a.resolved ? ' resolved' : '');
    card.dataset.annotationId = a.id;
    var meta = document.createElement('div');
    meta.className = 'annotation-meta';
    var author = document.createElement('span');
    author.className = 'author';
    author.textContent = a.creator ? a.creator.name : 'Unknown';
    meta.appendChild(author);
    var date = document.createElement('span');
    date.textContent = a.created ? new Date(a.created).toLocaleDateString() : '';
    meta.appendChild(date);
    card.appendChild(meta);
    if (a.target && a.target.selector) {
      for (var i = 0; i < a.target.selector.length; i++) {
        var sel = a.target.selector[i];
        if (sel.type === 'TextQuoteSelector' && sel.exact) {
          var quote = document.createElement('div');
          quote.className = 'annotation-quote';
          quote.textContent = sel.exact.length > 80 ? sel.exact.slice(0, 80) + '...' : sel.exact;
          card.appendChild(quote);
          break;
        }
      }
    }
    var body = document.createElement('div');
    body.className = 'annotation-body';
    body.innerHTML = simpleMarkdown(a.body ? a.body.value : '');
    card.appendChild(body);
    card.addEventListener('click', function () {
      var el = document.querySelector('[data-source].annotation-highlight[data-annotation-id="' + a.id + '"]');
      if (el) el.scrollIntoView({ behavior: 'smooth', block: 'center' });
    });
    return card;
  }

  function highlightAnnotations(annotations) {
    for (var i = 0; i < annotations.length; i++) {
      var a = annotations[i];
      if (!a.target || !a.target.selector) continue;
      for (var j = 0; j < a.target.selector.length; j++) {
        if (a.target.selector[j].type === 'TextQuoteSelector') {
          highlightTextInContent(a.target.selector[j].exact, a.id);
          break;
        }
      }
    }
  }

  function highlightTextInContent(text, annotationId) {
    var content = document.getElementById('rfd-content');
    if (!content) return;
    var elements = content.querySelectorAll('[data-source]');
    for (var i = 0; i < elements.length; i++) {
      if (elements[i].textContent.indexOf(text) !== -1) {
        elements[i].classList.add('annotation-highlight');
        elements[i].dataset.annotationId = annotationId;
        (function (el, aid) {
          el.addEventListener('click', function () {
            var card = document.querySelector('.annotation-card[data-annotation-id="' + aid + '"]');
            if (card) {
              document.querySelectorAll('.annotation-card.active').forEach(function (c) { c.classList.remove('active'); });
              card.classList.add('active');
              card.scrollIntoView({ behavior: 'smooth', block: 'center' });
            }
          });
        })(elements[i], annotationId);
        break;
      }
    }
  }

  // ---------------------------------------------------------------------------
  // Text selection for annotation creation
  // ---------------------------------------------------------------------------

  function setupTextSelection() {
    var content = document.getElementById('rfd-content');
    var popover = document.getElementById('annotation-popover');
    if (!content || !popover) return;
    content.addEventListener('mouseup', function () {
      var selection = window.getSelection();
      if (!selection || selection.isCollapsed) { popover.style.display = 'none'; return; }
      var text = selection.toString().trim();
      if (!text || text.length < 3 || !getToken()) return;
      var range = selection.getRangeAt(0);
      var rect = range.getBoundingClientRect();
      popover.style.display = 'block';
      popover.style.top = (rect.bottom + window.scrollY + 8) + 'px';
      popover.style.left = Math.max(8, rect.left + window.scrollX - 100) + 'px';
      popover.dataset.selectedText = text;
    });
    var cancelBtn = document.getElementById('annotation-cancel');
    if (cancelBtn) cancelBtn.addEventListener('click', function () { popover.style.display = 'none'; document.getElementById('annotation-input').value = ''; });
    var submitBtn = document.getElementById('annotation-submit');
    if (submitBtn) submitBtn.addEventListener('click', function () {
      var text = popover.dataset.selectedText;
      var comment = document.getElementById('annotation-input').value.trim();
      if (!text || !comment) return;
      var rfdNumber = window.__RFD_NUMBER__;
      var annotation = {
        '@context': 'http://www.w3.org/ns/anno.jsonld', type: 'Annotation',
        id: 'urn:openrfd:' + rfdNumber + ':' + crypto.randomUUID(),
        creator: { type: 'Person', name: (getUser() || {}).login || 'anonymous' },
        created: new Date().toISOString(), motivation: 'commenting',
        body: { type: 'TextualBody', value: comment, format: 'text/markdown' },
        target: { type: 'SpecificResource', source: 'rfd/' + rfdNumber + '/README.md', selector: [{ type: 'TextQuoteSelector', exact: text }] }
      };
      if (window.__ANNOTATIONS__) { window.__ANNOTATIONS__.push(annotation); renderAnnotations(window.__ANNOTATIONS__); }
      popover.style.display = 'none';
      document.getElementById('annotation-input').value = '';
    });
  }

  // ---------------------------------------------------------------------------
  // Live PR Discussion
  // ---------------------------------------------------------------------------

  async function loadPrComments() {
    if (!window.__RFD_DISCUSSION__ || !getToken()) return;
    var match = window.__RFD_DISCUSSION__.match(/github\.com\/([^/]+)\/([^/]+)\/pull\/(\d+)/);
    if (!match) return;
    var comments = await githubFetch('/repos/' + match[1] + '/' + match[2] + '/pulls/' + match[3] + '/comments');
    if (!comments || !Array.isArray(comments)) return;
    var section = document.getElementById('pr-comments');
    var list = document.getElementById('pr-comment-list');
    if (!section || !list) return;
    section.style.display = 'block';
    list.innerHTML = '';
    for (var i = 0; i < comments.length; i++) {
      var c = comments[i];
      var card = document.createElement('div');
      card.className = 'pr-comment-card';
      var meta = document.createElement('div');
      meta.className = 'annotation-meta';
      meta.innerHTML = '<span class="author">' + escapeHtml((c.user || {}).login || 'unknown') + '</span> <span>' + new Date(c.created_at).toLocaleDateString() + '</span> <span class="source-tag">via PR</span>';
      card.appendChild(meta);
      var body = document.createElement('div');
      body.className = 'annotation-body';
      body.innerHTML = simpleMarkdown(c.body || '');
      card.appendChild(body);
      list.appendChild(card);
    }
  }

  // ---------------------------------------------------------------------------
  // Search (Cmd+K)
  // ---------------------------------------------------------------------------

  var searchIndex = null;

  async function loadSearchIndex() {
    try { var res = await fetch('/search-index.json'); searchIndex = await res.json(); } catch (e) { searchIndex = []; }
  }

  function setupSearch() {
    var modal = document.getElementById('search-modal');
    var input = document.getElementById('modal-search-input');
    var headerInput = document.getElementById('search-input');
    var results = document.getElementById('search-results');
    if (!modal || !input || !results) return;
    document.addEventListener('keydown', function (e) {
      if ((e.metaKey || e.ctrlKey) && e.key === 'k') { e.preventDefault(); modal.style.display = modal.style.display === 'none' ? 'flex' : 'none'; if (modal.style.display === 'flex') { input.focus(); input.value = ''; results.innerHTML = ''; } }
      if (e.key === 'Escape') modal.style.display = 'none';
    });
    modal.addEventListener('click', function (e) { if (e.target === modal) modal.style.display = 'none'; });
    if (headerInput) headerInput.addEventListener('focus', function () { modal.style.display = 'flex'; input.focus(); headerInput.blur(); });
    input.addEventListener('input', function () {
      var q = input.value.trim().toLowerCase();
      if (!q || !searchIndex) { results.innerHTML = ''; return; }
      var matches = searchIndex.filter(function (entry) {
        return entry.title.toLowerCase().indexOf(q) !== -1 || entry.body.toLowerCase().indexOf(q) !== -1 || entry.number.indexOf(q) !== -1 || entry.authors.toLowerCase().indexOf(q) !== -1;
      }).slice(0, 20);
      results.innerHTML = '';
      for (var i = 0; i < matches.length; i++) {
        var m = matches[i];
        var div = document.createElement('div');
        div.className = 'search-result';
        div.innerHTML = '<span class="result-number">RFD ' + escapeHtml(m.number) + '</span> ' + escapeHtml(m.title) + ' <span class="result-state">' + escapeHtml(m.state) + '</span>';
        (function (num) { div.addEventListener('click', function () { window.location.href = '/rfd/' + num + '/'; }); })(m.number);
        results.appendChild(div);
      }
    });
  }

  // ---------------------------------------------------------------------------
  // Index page
  // ---------------------------------------------------------------------------

  function setupIndexPage() {
    var table = document.getElementById('rfd-table');
    var filter = document.getElementById('state-filter');
    if (!table) return;
    if (filter) {
      filter.addEventListener('change', function () {
        var val = filter.value;
        var rows = table.querySelectorAll('tbody tr');
        for (var i = 0; i < rows.length; i++) { rows[i].style.display = (!val || rows[i].dataset.state === val) ? '' : 'none'; }
      });
    }
    var headers = table.querySelectorAll('th.sortable');
    var sortCol = null, sortAsc = true;
    headers.forEach(function (th) {
      th.addEventListener('click', function () {
        var col = th.dataset.sort;
        if (sortCol === col) { sortAsc = !sortAsc; } else { sortCol = col; sortAsc = true; }
        var tbody = table.querySelector('tbody');
        var rows = Array.from(tbody.querySelectorAll('tr'));
        var colIdx = Array.from(th.parentElement.children).indexOf(th);
        rows.sort(function (a, b) {
          var at = (a.children[colIdx] || {}).textContent || '';
          var bt = (b.children[colIdx] || {}).textContent || '';
          var cmp = at.trim().localeCompare(bt.trim(), undefined, { numeric: true });
          return sortAsc ? cmp : -cmp;
        });
        for (var i = 0; i < rows.length; i++) tbody.appendChild(rows[i]);
      });
    });
  }

  // ---------------------------------------------------------------------------
  // Auth UI
  // ---------------------------------------------------------------------------

  function setupAuthUI() {
    var loginBtn = document.getElementById('login-btn');
    var userInfo = document.getElementById('user-info');
    var clientId = window.__GITHUB_CLIENT_ID__;
    if (!loginBtn || !clientId) return;
    var user = getUser();
    if (user) {
      loginBtn.style.display = 'none';
      if (userInfo) { userInfo.style.display = 'inline'; userInfo.textContent = user.login; userInfo.style.cursor = 'pointer'; userInfo.title = 'Click to sign out'; userInfo.addEventListener('click', function () { if (confirm('Sign out?')) { clearAuth(); window.location.reload(); } }); }
    } else {
      loginBtn.style.display = 'inline-flex';
      loginBtn.addEventListener('click', async function () { loginBtn.disabled = true; loginBtn.textContent = 'Signing in...'; var token = await deviceFlowLogin(clientId); if (token) { window.location.reload(); } else { loginBtn.disabled = false; loginBtn.textContent = 'Sign in with GitHub'; } });
    }
  }

  // ---------------------------------------------------------------------------
  // Utilities
  // ---------------------------------------------------------------------------

  function escapeHtml(str) { var d = document.createElement('div'); d.textContent = str; return d.innerHTML; }
  function simpleMarkdown(text) {
    return escapeHtml(text).replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>').replace(/\*(.+?)\*/g, '<em>$1</em>').replace(/`(.+?)`/g, '<code>$1</code>').replace(/\[(.+?)\]\((.+?)\)/g, '<a href="$2">$1</a>').replace(/\n/g, '<br>');
  }

  // ---------------------------------------------------------------------------
  // Init
  // ---------------------------------------------------------------------------

  document.addEventListener('DOMContentLoaded', function () {
    loadSearchIndex();
    setupSearch();
    setupIndexPage();
    setupAuthUI();
    if (window.__ANNOTATIONS__) { renderAnnotations(window.__ANNOTATIONS__); setupTextSelection(); loadPrComments(); }
  });
})();
