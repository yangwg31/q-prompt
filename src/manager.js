import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { save, open } from '@tauri-apps/plugin-dialog';
import { readTextFile, writeTextFile } from '@tauri-apps/plugin-fs';

let prompts = [];
let editingId = null;
let currentTab = 'list';

const listEl = document.getElementById('list');
const listWrap = document.getElementById('list-wrap');
const shortcutTab = document.getElementById('shortcut-tab');
const searchEl = document.getElementById('search');
const overlay = document.getElementById('modal-overlay');
const modalTitle = document.getElementById('modal-title');
const modalName = document.getElementById('modal-name');
const modalContent = document.getElementById('modal-content');

// ── Load & Render ──

async function loadPrompts() {
  prompts = await invoke('get_prompts');
  prompts.sort((a, b) => a.sort_order - b.sort_order);
  render();
}

function filterPrompts() {
  const q = searchEl.value.toLowerCase();
  return prompts.filter(p =>
    p.name.toLowerCase().includes(q) || p.content.toLowerCase().includes(q)
  );
}

function render() {
  const items = filterPrompts();
  listEl.innerHTML = items.map(p => {
    const summary = p.content.replace(/[\n\r]/g, ' ').slice(0, 30) + (p.content.length > 30 ? '...' : '');
    return `
      <div class="row" data-id="${p.id}" draggable="true">
        <span class="drag-handle">≡</span>
        <span class="name">${p.name}</span>
        <span class="summary">${summary}</span>
        <span class="count">${p.useCount}次</span>
        <span class="actions">
          <span class="edit-btn" data-action="edit">✎</span>
          <span class="del-btn" data-action="delete">✕</span>
        </span>
      </div>
    `;
  }).join('');
}

// ── Tab switching ──

document.querySelectorAll('.tab').forEach(t => {
  t.addEventListener('click', () => {
    document.querySelectorAll('.tab').forEach(x => x.classList.remove('active'));
    t.classList.add('active');
    currentTab = t.dataset.tab;
    if (currentTab === 'list') {
      listWrap.classList.remove('hidden');
      shortcutTab.classList.remove('show');
    } else {
      listWrap.classList.add('hidden');
      shortcutTab.classList.add('show');
      renderShortcuts();
    }
  });
});

// ── Row click ──

listEl.addEventListener('click', (e) => {
  const row = e.target.closest('.row');
  if (!row) return;
  const id = row.dataset.id;
  if (e.target.dataset.action === 'edit' || e.target.closest('.edit-btn')) {
    editPrompt(id);
  } else if (e.target.dataset.action === 'delete' || e.target.closest('.del-btn')) {
    deletePrompt(id);
  } else {
    editPrompt(id);
  }
});

// ── Double-click ──

listEl.addEventListener('dblclick', (e) => {
  const row = e.target.closest('.row');
  if (!row) return;
  const p = prompts.find(p => p.id === row.dataset.id);
  if (p) alert(`名称: ${p.name}\n\n内容:\n${p.content}`);
});

// ── Drag & Drop ──

let dragSrcId = null;

listEl.addEventListener('dragstart', (e) => {
  const row = e.target.closest('.row');
  if (!row) return;
  dragSrcId = row.dataset.id;
  row.style.opacity = '0.5';
});

listEl.addEventListener('dragend', () => {
  document.querySelectorAll('.row').forEach(r => r.style.opacity = '1');
  dragSrcId = null;
});

listEl.addEventListener('dragover', (e) => e.preventDefault());

listEl.addEventListener('drop', async (e) => {
  e.preventDefault();
  const row = e.target.closest('.row');
  if (!row || !dragSrcId || row.dataset.id === dragSrcId) return;
  const ids = prompts.map(p => p.id);
  const from = ids.indexOf(dragSrcId);
  const to = ids.indexOf(row.dataset.id);
  if (from >= 0 && to >= 0) {
    ids.splice(from, 1);
    ids.splice(to, 0, dragSrcId);
    await invoke('reorder_prompts', { orderedIds: ids });
    await loadPrompts();
  }
});

// ── Modal ──

function editPrompt(id) {
  const p = prompts.find(p => p.id === id);
  if (!p) return;
  editingId = id;
  modalTitle.textContent = '编辑提示词';
  modalName.value = p.name;
  modalContent.value = p.content;
  overlay.classList.add('show');
  modalName.focus();
}

document.getElementById('add-btn').addEventListener('click', () => {
  editingId = null;
  modalTitle.textContent = '新增提示词';
  modalName.value = '';
  modalContent.value = '';
  overlay.classList.add('show');
  modalName.focus();
});

document.getElementById('modal-cancel').addEventListener('click', () => {
  overlay.classList.remove('show');
});

document.getElementById('modal-save').addEventListener('click', async () => {
  const name = modalName.value.trim();
  const content = modalContent.value.trim();
  if (!name || !content) return;
  if (editingId) {
    await invoke('update_prompt', { id: editingId, name, content });
  } else {
    await invoke('add_prompt', { name, content });
  }
  overlay.classList.remove('show');
  await loadPrompts();
});

overlay.addEventListener('click', (e) => {
  if (e.target === overlay) overlay.classList.remove('show');
});

// ── Delete ──

async function deletePrompt(id) {
  const p = prompts.find(p => p.id === id);
  if (!p) return;
  if (!confirm(`确定删除「${p.name}」吗？\n\n删除后可在:\n%APPDATA%/q-prompt/deleted_backup.json\n中找到备份。`)) return;
  await invoke('delete_prompt', { id });
  await loadPrompts();
}

// ── Search ──

searchEl.addEventListener('input', render);

// ── Export / Import ──

document.getElementById('export-btn').addEventListener('click', async () => {
  const json = await invoke('export_prompts');
  const fn = await save({ defaultPath: 'q-prompt-export.json', filters: [{ name: 'JSON', extensions: ['json'] }] });
  if (fn) {
    await writeTextFile(fn, json);
  }
});

document.getElementById('import-btn').addEventListener('click', async () => {
  const sel = await open({ filters: [{ name: 'JSON', extensions: ['json'] }] });
  if (sel) {
    const text = await readTextFile(sel);
    const count = await invoke('import_prompts', { json: text });
    alert(`已导入 ${count} 条提示词`);
    await loadPrompts();
  }
});

// ── Shortcuts Tab ──

const shortcutLabels = {
  toggle_bar: '切换悬浮条', insert_1: '插入第1项', insert_2: '插入第2项',
  insert_3: '插入第3项', insert_4: '插入第4项', insert_5: '插入第5项',
  insert_6: '插入第6项', insert_7: '插入第7项', insert_8: '插入第8项',
  quick_save: '快速保存',
};

async function renderShortcuts() {
  const keys = Object.keys(shortcutLabels);
  const rows = [];
  for (const key of keys) {
    const current = await invoke('get_shortcut', { key });
    rows.push({ key, label: shortcutLabels[key], current });
  }
  shortcutTab.innerHTML = `
    <h3 style="margin-bottom:12px;color:#ddd;">快捷键配置</h3>
    ${rows.map(r => `
      <div class="sc-row">
        <span class="sc-name">${r.label}</span>
        <span class="sc-key">${r.current}</span>
        <button data-sc-key="${r.key}">修改</button>
      </div>
    `).join('')}
  `;

  shortcutTab.querySelectorAll('button[data-sc-key]').forEach(btn => {
    btn.addEventListener('click', async () => {
      const key = btn.dataset.scKey;
      const current = await invoke('get_shortcut', { key });
      const newShortcut = prompt(`为「${shortcutLabels[key]}」输入新快捷键：\n当前: ${current}\n示例: Ctrl+Shift+Q`, current);
      if (!newShortcut || newShortcut === current) return;
      try {
        await invoke('update_shortcut', { key, newShortcut });
        await renderShortcuts();
      } catch (e) {
        alert('快捷键注册失败: ' + e);
      }
    });
  });
}

// ── Keyboard ──

document.addEventListener('keydown', (e) => {
  if (e.key === 'Escape') {
    if (overlay.classList.contains('show')) {
      overlay.classList.remove('show');
    } else {
      getCurrentWindow().close();
    }
  }
  if ((e.ctrlKey || e.metaKey) && e.key === 'f') {
    e.preventDefault();
    searchEl.focus();
  }
  if (e.key === 'ArrowUp' || e.key === 'ArrowDown') {
    if (document.activeElement === searchEl) return;
    const rows = [...listEl.querySelectorAll('.row')];
    if (!rows.length) return;
    const selected = listEl.querySelector('.row.selected');
    const idx = selected ? rows.indexOf(selected) : -1;
    if (idx >= 0) selected.classList.remove('selected');
    let next = e.key === 'ArrowDown' ? Math.min(idx + 1, rows.length - 1) : Math.max(idx - 1, 0);
    if (idx < 0) next = 0;
    rows[next].classList.add('selected');
    rows[next].scrollIntoView({ block: 'nearest' });
  }
  if (e.key === 'Enter' && !overlay.classList.contains('show')) {
    const sel = listEl.querySelector('.row.selected');
    if (sel) editPrompt(sel.dataset.id);
  }
  if (e.key === 'Delete' && !overlay.classList.contains('show')) {
    const sel = listEl.querySelector('.row.selected');
    if (sel) deletePrompt(sel.dataset.id);
  }
});

// ── Init ──
await loadPrompts();
const win = getCurrentWindow();
await win.show();
