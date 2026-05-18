let allRecipes = [];
    let activeSlug = null;
    let categories = new Map(); // key -> label

    // --- API ---
    async function fetchRecipes() {
      const res = await fetch('/api/recipes');
      const data = await res.json();
      allRecipes = data.recipes || [];

      // Build categories map from existing recipes
      categories.clear();
      allRecipes.forEach(r => {
        if (r.category) {
          const label = r.category.charAt(0).toUpperCase() + r.category.slice(1);
          categories.set(r.category, label);
        }
      });

      renderTree();
    }

    async function fetchRecipe(slug) {
      const res = await fetch(`/api/recipes/${slug}`);
      if (!res.ok) return null;
      return res.json();
    }

    async function saveRecipe(e) {
      e.preventDefault();
      const originalSlug = document.getElementById('form-slug').value;
      const customSlug = document.getElementById('form-slug-edit').value.trim();
      const catKey = document.getElementById('form-category').value;
      const catLabel = categories.get(catKey) || catKey.charAt(0).toUpperCase() + catKey.slice(1);

      const data = {
        title: document.getElementById('form-title').value,
        category: catKey,
        excerpt: document.getElementById('form-excerpt').value || null,
        prep_time: intVal('form-prepTime'),
        cook_time: intVal('form-cookTime'),
        servings: strVal('form-servings'),
        difficulty: document.getElementById('form-difficulty').value || null,
        ingredients: collectIngredients(),
        ingredients2_title: document.getElementById('form-ingredients2-title').value || null,
        ingredients2: collectIngredients2(),
        notes: document.getElementById('form-notes').value || null,
        legende: document.getElementById('form-legende').value || null,
        source_image: document.getElementById('form-sourceImage').value || null,
        steps: collectSteps(),
      };

      // If editing and slug changed, we need to delete old + create new
      if (originalSlug && customSlug && customSlug !== originalSlug) {
        // Delete old, then create with new slug
        await fetch(`/api/recipes/${originalSlug}`, { method: 'DELETE' });
        const res = await fetch('/api/recipes', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ ...data, slug: customSlug }),
        });
        if (!res.ok) {
          const err = await res.json();
          alert('Erreur: ' + (err.error || res.statusText));
          return;
        }
      } else {
        const slug = customSlug || originalSlug;
        const url = slug ? `/api/recipes/${slug}` : '/api/recipes';
        const res = await fetch(url, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(data),
        });

        if (!res.ok) {
          const err = await res.json();
          alert('Erreur: ' + (err.error || res.statusText));
          return;
        }
      }

      await fetchRecipes();
      // Stay on the recipe — update slug if renamed, reset dirty state
      activeSlug = customSlug || originalSlug;
      document.getElementById('form-slug').value = activeSlug;
      document.getElementById('form-slug-edit').value = activeSlug;
      loadManuscriptImage(activeSlug);
      checkGitStatus(); // refresh git indicators
    }

    async function deleteRecipe() {
      const slug = document.getElementById('form-slug').value;
      if (!slug) return;
      if (!confirm('Supprimer cette recette ?')) return;

      const res = await fetch(`/api/recipes/${slug}`, { method: 'DELETE' });
      if (res.ok) {
        await fetchRecipes();
        cancelEdit();
        checkGitStatus(); // refresh git indicators
      }
    }

    // --- Tree view ---
    function renderTree() {
      const search = document.getElementById('search').value.toLowerCase();
      const filtered = allRecipes.filter(r =>
        r.title.toLowerCase().includes(search) ||
        r.category.toLowerCase().includes(search)
      );

      // Group by NORMALIZED key (lowercase, trim trailing s) to detect non-standard categories
      const normalized = new Map(); // normalizedKey -> { keys: Set, label, recipes: [] }
      filtered.forEach(r => {
        const rawKey = r.category || 'autres';
        const catLabel = r.category || 'Autres';
        const normKey = rawKey.toLowerCase().replace(/s$/, '');

        if (!normalized.has(normKey)) {
          normalized.set(normKey, { keys: new Set(), label: catLabel, recipes: [] });
        }
        const group = normalized.get(normKey);
        group.keys.add(rawKey);
        group.recipes.push(r);
        // Use the most common label
        if (!group.label || group.label === rawKey) group.label = catLabel;
      });

      // Sort categories alphabetically, recipes by title
      const sortedCats = [...normalized.entries()].sort((a, b) => a[1].label.localeCompare(b[1].label, 'fr'));
      sortedCats.forEach(([_, data]) => {
        data.recipes.sort((a, b) => a.title.localeCompare(b.title, 'fr'));
      });

      const container = document.getElementById('recipe-tree');
      const total = allRecipes.length;

      let html = `
        <div class="tree-toggle flex items-center justify-between px-4 py-2 border-b border-stone-100" onclick="toggleTreeAll()">
          <span class="text-xs font-medium text-stone-500 uppercase tracking-wider">Toutes les recettes</span>
          <span class="text-xs text-stone-400">${total}</span>
        </div>
      `;

      sortedCats.forEach(([normKey, data]) => {
        const recipes = data.recipes;
        const cat = data.label;
        const catId = `cat-${normKey.replace(/[^a-z0-9]/gi, '-')}`;
        const hasMultipleKeys = data.keys.size > 1;
        const keyList = [...data.keys].join(', ');

        html += `
          <div>
            <div class="tree-toggle flex items-center justify-between px-4 py-2 border-b border-stone-50"
                 onclick="toggleTree('${catId}')" title="${hasMultipleKeys ? 'Clés: ' + esc(keyList) : ''}">
              <div class="flex items-center gap-2">
                <span class="chevron" id="chevron-${catId}">▼</span>
                <span class="text-sm font-medium text-stone-700">${esc(cat)}</span>
                ${hasMultipleKeys ? '<span class="text-xs text-amber-500" title="Catégories non-normalisées: ' + esc(keyList) + '">⚠️</span>' : ''}
              </div>
              <span class="text-xs text-stone-400">${recipes.length}</span>
            </div>
            <div class="tree-children" id="children-${catId}">
              ${recipes.map(r => `
                <button onclick="editRecipe('${r.slug}')"
                  class="tree-recipe w-full text-left pl-10 pr-4 py-2 border-b border-stone-50 transition-colors ${r.slug === activeSlug ? 'active' : ''}">
                  <div class="flex items-center gap-1.5">
                    ${dirtySlugs.has(r.slug) ? '<span class="w-2 h-2 rounded-full bg-amber-400 flex-shrink-0" title="Non poussé sur GitHub"></span>' : ''}
                    <span class="text-sm text-stone-800">${esc(r.title)}</span>
                  </div>
                </button>
              `).join('')}
            </div>
          </div>
        `;
      });

      container.innerHTML = html;
      updateCategoryDropdown();
    }

    function toggleTree(catId) {
      const children = document.getElementById(`children-${catId}`);
      const chevron = document.getElementById(`chevron-${catId}`);
      if (children && chevron) {
        children.classList.toggle('collapsed');
        chevron.classList.toggle('collapsed');
      }
    }

    function toggleTreeAll() {
      const allChildren = document.querySelectorAll('.tree-children');
      const allChevrons = document.querySelectorAll('.chevron');
      const allCollapsed = [...allChildren].every(c => c.classList.contains('collapsed'));

      allChildren.forEach(c => c.classList.toggle('collapsed', !allCollapsed));
      allChevrons.forEach(c => c.classList.toggle('collapsed', !allCollapsed));
    }

    function filterRecipes() { renderTree(); }

    // --- Category dropdown ---
    function updateCategoryDropdown() {
      const select = document.getElementById('form-category');
      const currentVal = select.value;
      select.innerHTML = '';

      // Add existing categories
      const sorted = [...categories.entries()].sort((a, b) => a[1].localeCompare(b[1], 'fr'));
      sorted.forEach(([key, label]) => {
        const opt = document.createElement('option');
        opt.value = key;
        opt.textContent = label;
        select.appendChild(opt);
      });

      // Restore selection
      if (currentVal && [...categories.keys()].includes(currentVal)) {
        select.value = currentVal;
      }
    }

    function showAddCategory() {
      document.getElementById('new-category-row').classList.remove('hidden');
      document.getElementById('add-cat-btn').classList.add('hidden');
      document.getElementById('form-new-category').focus();
    }

    function hideAddCategory() {
      document.getElementById('new-category-row').classList.add('hidden');
      document.getElementById('add-cat-btn').classList.remove('hidden');
      document.getElementById('form-new-category').value = '';
    }

    function confirmNewCategory() {
      const input = document.getElementById('form-new-category').value.trim();
      if (!input) return;

      const key = input.toLowerCase()
        .replace(/[^a-z0-9àâäéèêëîïôöùûüç\s-]/gi, '')
        .replace(/\s+/g, '-');
      // Use input as-is for label (capitalized)
      const label = input.charAt(0).toUpperCase() + input.slice(1);

      if (!key) return;

      categories.set(key, label);
      updateCategoryDropdown();
      document.getElementById('form-category').value = key;
      hideAddCategory();
    }

    // --- Form ---
    function showNewRecipe() {
      activeSlug = null;
      document.getElementById('form-slug').value = '';
      document.getElementById('form-slug-edit').value = '';
      document.getElementById('recipe-form').reset();
      document.getElementById('ingredients-raw').value = '';
      document.getElementById('form-ingredients2-title').value = '';
      document.getElementById('ingredients2-section').classList.add('hidden');
      document.getElementById('add-i2-btn').classList.remove('hidden');
      document.getElementById('steps-raw').value = '';
      document.getElementById('delete-btn').classList.add('hidden');
      hideAddCategory();
      // Use raw textarea mode for new recipes
      // Reset manuscript
      document.getElementById('manuscript-img').classList.add('hidden');
      document.getElementById('manuscript-placeholder').classList.remove('hidden');
      showForm();
      renderTree(); // refresh active state
    }

    async function editRecipe(slug) {
      const recipe = await fetchRecipe(slug);
      if (!recipe) return;

      activeSlug = slug;
      document.getElementById('form-slug').value = recipe.slug;
      document.getElementById('form-slug-edit').value = recipe.slug || '';
      document.getElementById('form-title').value = recipe.title || '';
      document.getElementById('form-category').value = recipe.category || 'desserts';
      document.getElementById('form-difficulty').value = recipe.difficulty || '';
      document.getElementById('form-prepTime').value = recipe.prepTime || '';
      document.getElementById('form-cookTime').value = recipe.cookTime || '';
      document.getElementById('form-servings').value = recipe.servings || '';
      document.getElementById('form-excerpt').value = recipe.excerpt || '';
      document.getElementById('form-notes').value = recipe.notes || '';
      document.getElementById('form-legende').value = recipe.legende || '';
      document.getElementById('form-sourceImage').value = recipe.sourceImage || '';

      document.getElementById('ingredients-raw').value = (recipe.ingredients || []).join('\n');

      document.getElementById('form-ingredients2-title').value = recipe.ingredients2Title || '';
      document.getElementById('ingredients2-raw').value = (recipe.ingredients2 || []).join('\n');

      // Show/hide ingredients2 section based on data
      const hasI2 = recipe.ingredients2Title || (recipe.ingredients2 && recipe.ingredients2.length > 0);
      document.getElementById('ingredients2-section').classList.toggle('hidden', !hasI2);
      document.getElementById('add-i2-btn').classList.toggle('hidden', hasI2);

      // Manuscript image
      loadManuscriptImage(recipe.slug);
      loadImageGroup(recipe.slug);

      document.getElementById('steps-raw').value = (recipe.steps || []).join('\n');

      document.getElementById('delete-btn').classList.remove('hidden');
      hideAddCategory();
      showForm();
      renderTree(); // refresh active state
    }

    function showForm() {
      document.getElementById('empty-state').classList.add('hidden');
      document.getElementById('form-container').classList.remove('hidden');
    }

    function cancelEdit() {
      activeSlug = null;
      imageGroup = [];
      ocrSelectedSlug = null;
      document.getElementById('image-group').classList.add('hidden');
      document.getElementById('empty-state').classList.remove('hidden');
      document.getElementById('form-container').classList.add('hidden');
      renderTree();
    }

    // --- Dynamic lists ---






    function collectIngredients() {
      return document.getElementById('ingredients-raw').value.split('\n').filter(v => v.trim());
    }



    function collectIngredients2() {
      const items = document.getElementById('ingredients2-raw').value.split('\n').filter(v => v.trim());
      return items.length > 0 ? items : null;
    }

    function collectSteps() {
      return document.getElementById('steps-raw').value.split('\n').filter(v => v.trim());
    }

    // --- Helpers ---
    function intVal(id) {
      const v = document.getElementById(id).value;
      return v ? String(parseInt(v)) : null;
    }
    function strVal(id) {
      const v = document.getElementById(id).value.trim();
      return v || null;
    }

    function esc(s) {
      if (!s) return '';
      const d = document.createElement('div');
      d.textContent = s;
      return d.innerHTML;
    }

    // --- Ingredients 2 toggle ---
    function toggleIngredients2() {
      document.getElementById('ingredients2-section').classList.remove('hidden');
      document.getElementById('add-i2-btn').classList.add('hidden');
      document.getElementById('form-ingredients2-title').focus();
    }

    function hideIngredients2() {
      document.getElementById('ingredients2-section').classList.add('hidden');
      document.getElementById('add-i2-btn').classList.remove('hidden');
      document.getElementById('form-ingredients2-title').value = '';
    }

    // --- Manuscript image ---
    let ocrAvailable = false;

    function switchImageTab(tab) {
      document.getElementById('tab-image').classList.toggle('active', tab === 'image');
      document.getElementById('tab-ocr-tab').classList.toggle('active', tab === 'ocr');
      document.getElementById('image-panel').classList.toggle('hidden', tab !== 'image');
      document.getElementById('ocr-panel').classList.toggle('hidden', tab !== 'ocr');
    }

    function loadManuscriptImage(slug) {
      const img = document.getElementById('manuscript-img');
      const placeholder = document.getElementById('manuscript-placeholder');

      // Try loading image — slug may match a file in img/
      img.src = `/api/images/${slug}`;
      img.onload = () => {
        img.classList.remove('hidden');
        placeholder.classList.add('hidden');
        // Show OCR button if available (it's in the OCR tab)
        if (ocrAvailable) document.getElementById('ocr-btn').classList.remove('hidden');
      };
      img.onerror = () => {
        img.classList.add('hidden');
        placeholder.classList.remove('hidden');
        document.getElementById('ocr-btn').classList.add('hidden');
      };
    }

    async function runOcr() {
      // Use OCR-selected image, fallback to current slug
      const slug = ocrSelectedSlug || document.getElementById('form-slug').value || document.getElementById('form-slug-edit').value;
      if (!slug) return;

      const btn = document.getElementById('ocr-btn');
      const result = document.getElementById('ocr-result');
      const textEl = document.getElementById('ocr-text');
      const placeholderEl = document.getElementById('ocr-placeholder');

      btn.textContent = '⏳ Analyse en cours...';
      btn.disabled = true;

      try {
        const res = await fetch(`/api/ocr/${slug}`, { method: 'POST' });
        const data = await res.json();

        if (!res.ok) {
          alert('OCR erreur: ' + (data.error || 'Unknown error'));
          return;
        }

        textEl.textContent = data.text || '(aucun texte détecté)';
        result.classList.remove('hidden');
        placeholderEl.classList.add('hidden');
        document.getElementById('ocr-tab-badge').classList.remove('hidden');
      } catch (e) {
        alert('OCR erreur: ' + e.message);
      } finally {
        btn.textContent = '🔍 Relancer la transcription';
        btn.disabled = false;
      }
    }

    function copyOcrResult() {
      const text = document.getElementById('ocr-text').textContent;
      navigator.clipboard.writeText(text);
    }

    function closeOcrResult() {
      document.getElementById('ocr-result').classList.add('hidden');
    }

    // --- Orphan images ---
    let orphanImages = [];
    let imageGroup = []; // variants for current recipe
    let ocrSelectedSlug = null; // which variant is selected for OCR

    async function fetchOrphanImages() {
      try {
        const res = await fetch('/api/orphan-images');
        const data = await res.json();
        orphanImages = data.orphanImages || [];
      } catch (e) {
        orphanImages = [];
      }
      updateOrphanBadge();
      renderOrphanTree();
    }

    function updateOrphanBadge() {
      const badge = document.getElementById('orphan-count');
      if (orphanImages.length > 0) {
        badge.textContent = orphanImages.length;
        badge.classList.remove('hidden');
      } else {
        badge.classList.add('hidden');
      }
    }

    function renderOrphanTree() {
      const container = document.getElementById('orphan-tree');
      if (orphanImages.length === 0) {
        container.innerHTML = `
          <div class="flex flex-col items-center justify-center h-full text-stone-400 p-4">
            <span class="text-2xl mb-2">📷</span>
            <p class="text-sm text-center">Toutes les images sont associées à une recette</p>
          </div>
        `;
        return;
      }

      container.innerHTML = `
        <div class="p-3 space-y-3">
          <p class="text-xs text-stone-400 px-1">Images sans recette — cliquer pour créer</p>
          ${orphanImages.map(img => `
            <div class="orphan-card bg-white border border-stone-200" onclick="newRecipeFromImage('${esc(img.slug)}')">
              <img src="/api/images/${esc(img.slug)}" alt="${esc(img.slug)}" class="w-full aspect-[3/4] object-cover" />
              <div class="px-3 py-2 text-sm font-mono text-stone-600">${esc(img.slug)}</div>
            </div>
          `).join('')}
        </div>
      `;
    }

    async function newRecipeFromImage(slug) {
      showNewRecipe();
      document.getElementById('form-slug-edit').value = slug;
      loadManuscriptImage(slug);
      await loadImageGroup(slug);
    }

    async function loadImageGroup(slug) {
      try {
        const res = await fetch(`/api/images-group/${slug}`);
        const data = await res.json();
        imageGroup = data.images || [];
      } catch (e) {
        imageGroup = [];
      }

      // Show image group in left panel
      const groupEl = document.getElementById('image-group');
      if (imageGroup.length > 1) {
        const currentSlug = document.getElementById('form-slug-edit').value;
        groupEl.innerHTML = imageGroup.map(s => `
          <img src="/api/images/${esc(s)}" onclick="selectSlug('${esc(s)}')"
            class="h-16 w-12 object-cover rounded cursor-pointer border-2 transition-all ${s === currentSlug ? 'border-amber-500 ring-1 ring-amber-500' : 'border-stone-200 opacity-60 hover:opacity-100'}"
            title="${esc(s)}" />
        `).join('');
        groupEl.classList.remove('hidden');
      } else {
        groupEl.innerHTML = '';
        groupEl.classList.add('hidden');
      }

      // Setup OCR variants
      renderOcrVariants();
    }

    function selectSlug(slug) {
      document.getElementById('form-slug-edit').value = slug;
      loadManuscriptImage(slug);
      // Re-render thumbnails to update selection
      loadImageGroup(slug);
    }

    function renderOcrVariants() {
      const container = document.getElementById('ocr-variants');
      const section = document.getElementById('ocr-images');
      const btn = document.getElementById('ocr-btn');

      if (imageGroup.length > 1) {
        const currentSlug = document.getElementById('form-slug-edit').value;
        // Default OCR selection = current slug
        if (!ocrSelectedSlug || !imageGroup.includes(ocrSelectedSlug)) {
          ocrSelectedSlug = currentSlug;
        }
        container.innerHTML = imageGroup.map(s => `
          <img src="/api/images/${esc(s)}" onclick="selectOcrImage('${esc(s)}')"
            class="h-20 w-16 object-cover rounded cursor-pointer border-2 transition-all ${s === ocrSelectedSlug ? 'border-amber-500 ring-2 ring-amber-400' : 'border-stone-200 opacity-60 hover:opacity-100'}"
            title="${esc(s)}" />
        `).join('');
        section.classList.remove('hidden');
        if (ocrAvailable) btn.classList.remove('hidden');
      } else if (imageGroup.length === 1) {
        ocrSelectedSlug = imageGroup[0];
        section.classList.add('hidden');
        if (ocrAvailable) btn.classList.remove('hidden');
      } else {
        section.classList.add('hidden');
        btn.classList.add('hidden');
      }
    }

    function selectOcrImage(slug) {
      ocrSelectedSlug = slug;
      document.getElementById('form-sourceImage').value = slug;
      renderOcrVariants();
    }

    // --- Tabs ---
    let currentTab = 'recipes';
    let contentFiles = [];
    let activeContentPath = null;

    function switchTab(tab) {
      currentTab = tab;
      document.getElementById('tab-recipes').classList.toggle('active', tab === 'recipes');
      document.getElementById('tab-orphans').classList.toggle('active', tab === 'orphans');
      document.getElementById('tab-content').classList.toggle('active', tab === 'content');
      document.getElementById('recipe-tree').classList.toggle('hidden', tab !== 'recipes');
      document.getElementById('orphan-tree').classList.toggle('hidden', tab !== 'orphans');
      document.getElementById('content-panel').classList.toggle('hidden', tab !== 'content');

      if (tab === 'content') {
        document.getElementById('form-container').classList.add('hidden');
        if (activeContentPath) {
          document.getElementById('content-editor').classList.remove('hidden');
          document.getElementById('empty-state').classList.add('hidden');
        } else {
          showContentEmpty();
        }
        fetchContentFiles();
      } else {
        // Hide content editor when switching away
        document.getElementById('content-editor').classList.add('hidden');
        if (activeSlug) {
          document.getElementById('form-container').classList.remove('hidden');
          document.getElementById('empty-state').classList.add('hidden');
        } else {
          document.getElementById('empty-state').classList.remove('hidden');
          // Restore default empty state text
          const p = document.getElementById('empty-state').querySelector('p');
          if (p) p.textContent = 'Sélectionnez ou créez une recette';
        }
      }

      if (tab === 'orphans' && orphanImages.length === 0) {
        renderOrphanTree();
      }
    }

    // --- Content editor ---
    async function fetchContentFiles() {
      try {
        const res = await fetch('/api/content');
        const data = await res.json();
        contentFiles = data.files || [];
      } catch (e) {
        contentFiles = [];
      }
      renderContentPanel();
    }

    function renderContentPanel() {
      const container = document.getElementById('content-panel');

      if (contentFiles.length === 0) {
        container.innerHTML = `
          <div class="flex flex-col items-center justify-center h-full text-stone-400 p-4">
            <span class="text-2xl mb-2">📝</span>
            <p class="text-sm text-center">Aucun fichier texte trouvé</p>
            <p class="text-xs text-stone-300 mt-1">src/data/content/ est vide ou introuvable</p>
          </div>
        `;
        return;
      }

      const fileItems = contentFiles.map(f => {
        const isActive = f.path === activeContentPath;
        return `
          <button onclick="loadContentFile('${esc(f.path)}')"
            class="w-full text-left px-4 py-2 border-b border-stone-50 transition-colors ${isActive ? 'bg-amber-100 text-amber-900' : 'hover:bg-stone-50'}">
            <div class="flex items-center gap-2">
              <span class="text-xs">${f.path.endsWith('.md') ? '📄' : f.path.endsWith('.json') ? '📋' : '📄'}</span>
              <div class="min-w-0">
                <div class="text-sm ${isActive ? 'font-medium' : 'text-stone-700'} truncate">${esc(f.name)}</div>
                <div class="text-xs text-stone-400 truncate">${esc(f.path.split('/').slice(0, -1).join('/') || '/')}</div>
              </div>
            </div>
          </button>
        `;
      }).join('');

      container.innerHTML = `
        <div class="flex flex-col h-full">
          <div class="text-xs font-medium text-stone-500 uppercase tracking-wider px-4 py-2 border-b border-stone-100">
            Fichiers (${contentFiles.length})
          </div>
          <div class="flex-1 overflow-y-auto">
            ${fileItems}
          </div>
        </div>
      `;
    }

    function showContentEmpty() {
      document.getElementById('content-editor').classList.add('hidden');
      document.getElementById('empty-state').classList.remove('hidden');
      document.getElementById('empty-state').querySelector('p').textContent = 'Sélectionnez un fichier texte';
    }

    let contentFileContent = '';

    async function loadContentFile(path) {
      try {
        const res = await fetch(`/api/content/${path}`);
        if (!res.ok) { alert('Fichier introuvable'); return; }
        const data = await res.json();
        activeContentPath = path;
        contentFileContent = data.content || '';
        renderContentPanel();

        // Show editor in main
        document.getElementById('empty-state').classList.add('hidden');
        document.getElementById('form-container').classList.add('hidden');
        const editor = document.getElementById('content-editor');
        editor.classList.remove('hidden');
        document.getElementById('content-editor-title').textContent = path;
        document.getElementById('content-textarea').value = contentFileContent;
      } catch (e) {
        alert('Erreur: ' + e.message);
      }
    }

    async function saveContentFile() {
      if (!activeContentPath) return;
      const textarea = document.getElementById('content-textarea');
      if (!textarea) return;

      try {
        const res = await fetch(`/api/content/${activeContentPath}`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ content: textarea.value }),
        });
        if (!res.ok) { alert('Erreur de sauvegarde'); return; }
        contentFileContent = textarea.value;
        const saved = document.getElementById('content-saved');
        if (saved) {
          saved.classList.remove('hidden');
          setTimeout(() => saved.classList.add('hidden'), 2000);
        }
        checkGitStatus();
      } catch (e) {
        alert('Erreur: ' + e.message);
      }
    }

    // --- Git push ---
    let dirtySlugs = new Set();

    async function checkGitStatus() {
      try {
        const res = await fetch('/api/git-status');
        const data = await res.json();
        const btn = document.getElementById('git-push-btn');
        const count = document.getElementById('git-count');
        if (data.error) return;

        // Update dirty slugs
        dirtySlugs = new Set(data.dirtySlugs || []);

        if (data.changes > 0) {
          count.textContent = data.changes;
          btn.classList.remove('hidden');
        } else {
          btn.classList.add('hidden');
        }

        // Refresh tree to show/hide dirty indicators
        renderTree();
      } catch (e) {}
    }

    async function gitPush() {
      const btn = document.getElementById('git-push-btn');
      const count = document.getElementById('git-count');
      btn.disabled = true;
      btn.innerHTML = '⏳ Envoi...';

      try {
        const res = await fetch('/api/git-push', { method: 'POST' });
        const data = await res.json();
        if (data.ok) {
          btn.innerHTML = '✅ ' + (data.message || 'OK');
          setTimeout(() => checkGitStatus(), 1500);
        } else {
          alert('Erreur: ' + (data.error || 'Unknown error'));
          btn.innerHTML = '📤 <span id="git-count">' + count.textContent + '</span> fichier(s)';
        }
      } catch (e) {
        alert('Erreur: ' + e.message);
        btn.innerHTML = '📤 <span id="git-count">?</span> fichier(s)';
      }
      btn.disabled = false;
    }



    // --- Init ---
    fetchRecipes();
    fetchOrphanImages();
    initSiteLink();
    checkOcrStatus();
    checkGitStatus();
    checkForUpdate();
    // Check for updates every 5 minutes
    setInterval(checkForUpdate, 300000);

    async function checkOcrStatus() {
      try {
        const res = await fetch('/api/ocr-status');
        const data = await res.json();
        ocrAvailable = data.available === true;
        // If image already loaded and OCR available, show button
        if (ocrAvailable && !document.getElementById('manuscript-img').classList.contains('hidden')) {
          document.getElementById('ocr-btn').classList.remove('hidden');
        }
      } catch (e) {}
    }

    async function initSiteLink() {
      try {
        const res = await fetch('/api/site-url');
        const data = await res.json();
        if (data.siteUrl) {
          const link = document.getElementById('site-link');
          link.href = data.siteUrl;
          link.classList.remove('hidden');
        }
      } catch (e) {}
    }

    async function checkForUpdate() {
      try {
        const res = await fetch('/api/update-check');
        const data = await res.json();
        document.getElementById('version-label').textContent = 'v' + data.current;
        if (data.updateAvailable) {
          document.getElementById('update-version').textContent = data.latest;
          document.getElementById('update-btn').classList.remove('hidden');
        }
      } catch (e) {}
    }

    function downloadUpdate() {
      const btn = document.getElementById('update-btn');
      btn.textContent = '⏳ Téléchargement…';
      btn.disabled = true;
      // Restart the app to apply staged update — updater handles download on next launch
      fetch('/api/update-check');
      // The updater runs at startup, so just tell user to restart
      btn.textContent = '🔄 Redémarrez pour appliquer';
    }