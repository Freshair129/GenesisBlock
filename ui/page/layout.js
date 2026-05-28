/* 
 * GKS/MSP Unified Layout Script (layout.js)
 * Handles dynamic navigation injection, responsive states, and TOC sync
 */

document.addEventListener('DOMContentLoaded', () => {
  // 1. Pages Registry (Config for all pages in the suite)
  const PAGES = [
    {
      name: "Console Dashboard",
      label: "Console Dashboard",
      filename: "index.html",
      icon: `<svg class="w-4 h-4 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 12l2-2m0 0l7-7 7 7M5 10v10a1 1 0 001 1h3m10-11l2 2m-2-2v10a1 1 0 01-1 1h-3m-6 0a1 1 0 001-1v-4a1 1 0 011-1h2a1 1 0 011 1v4a1 1 0 001 1m-6 0h6"></path></svg>`
    },
    {
      name: "Core Architecture & Governance",
      label: "1. สถาปัตยกรรมหลัก",
      filename: "gks_msp_specification_page_1_core_architecture_governance.html",
      icon: `<svg class="w-4 h-4 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 002-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10"></path></svg>`
    },
    {
      name: "Memory Subsystem & Distillation",
      label: "2. ระบบความจำ",
      filename: "gks_msp_specification_page_2_memory_subsystem_distillation.html",
      icon: `<svg class="w-4 h-4 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6.253v13m0-13C10.832 5.477 9.246 5 7.5 5S4.168 5.477 3 6.253v13C4.168 18.477 5.754 18 7.5 18s3.332.477 4.5 1.253m0-13C13.168 5.477 14.754 5 16.5 5c1.747 0 3.332.477 4.5 1.253v13C19.832 18.477 18.247 18 16.5 18c-1.746 0-3.332.477-4.5 1.253"></path></svg>`
    },
    {
      name: "Universal Context Framework & Policy Engine",
      label: "3. นโยบาย & UCF",
      filename: "gks_msp_specification_page_3_universal_context_framework_policy_engine.html",
      icon: `<svg class="w-4 h-4 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z"></path></svg>`
    },
    {
      name: "Symbol Graph & Code Intelligence",
      label: "4. กราฟสัญลักษณ์",
      filename: "gks_msp_specification_page_4_symbol_graph_code_intelligence.html",
      icon: `<svg class="w-4 h-4 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 3.055A9.003 9.003 0 1020.945 13H11V3.055z"></path><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20.488 9H15V3.512A9.025 9.025 0 0120.488 9z"></path></svg>`
    },
    {
      name: "Agent Dispatch Pipeline",
      label: "5. สายพานเขียนโค้ด",
      filename: "gks_msp_specification_page_5_agent_dispatch_codegen_pipeline.html",
      icon: `<svg class="w-4 h-4 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 9l3 3-3 3m5 0h3M5 20h14a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z"></path></svg>`
    },
    {
      name: "Validation & QA Automation",
      label: "6. ระบบตรวจประเมิน",
      filename: "gks_msp_specification_page_6_validation_automation_qa.html",
      icon: `<svg class="w-4 h-4 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"></path></svg>`
    },
    {
      name: "Meta Learning Loop",
      label: "7. วงจรการเรียนรู้",
      filename: "gks_msp_specification_page_7_meta_learning_loop.html",
      icon: `<svg class="w-4 h-4 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 1121.21 15.21M17 11V5h-6M13.447 8.447L17 12"></path></svg>`
    },
    {
      name: "Interactive Backlog Board",
      label: "แผงควบคุมบอร์ด",
      filename: "interactive_backlog_board.html",
      icon: `<svg class="w-4 h-4 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2m-3 7h3m-3 4h3m-6-4h.01M9 16h.01"></path></svg>`
    }
  ];

  // 2. Identify Current Page Name
  const currentPath = window.location.pathname;
  const currentFilename = currentPath.substring(currentPath.lastIndexOf('/') + 1);
  const activePage = PAGES.find(p => currentFilename.includes(p.filename)) || PAGES[0];
  const isBacklogPage = activePage.filename === "interactive_backlog_board.html";

  // 3. Clean up Old Headers & Sidebars
  const oldHeader = document.querySelector('header');
  if (oldHeader) oldHeader.style.display = 'none';

  const oldSidebar = document.querySelector('div.hidden.lg\\:block.fixed') || document.querySelector('aside');
  if (oldSidebar && oldSidebar.id !== 'gks-sidebar') {
    oldSidebar.style.display = 'none';
  }

  // 4. Inject Unified Navbar
  const navbar = document.createElement('nav');
  navbar.className = 'gks-navbar flex items-center justify-between px-6 py-3 text-slate-800';
  navbar.innerHTML = `
    <div class="flex items-center gap-3">
      <!-- Mobile hamburger toggle button -->
      <button id="gks-mobile-toggle" class="lg:hidden p-1 hover:bg-slate-100 rounded transition-colors text-slate-500 hover:text-slate-900">
        <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 12h16M4 18h16"></path></svg>
      </button>
      
      <!-- Logo Badge -->
      <div class="gks-logo-badge text-white p-1.5 rounded-lg flex items-center justify-center cursor-pointer" onclick="window.location.href='index.html'" title="Return to Dashboard">
        <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 002-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10"></path></svg>
      </div>
      
      <!-- Brand Titles -->
      <div class="flex flex-col">
        <span class="text-sm font-extrabold tracking-tight text-slate-900 leading-none">GKS / MSP Platform</span>
        <span class="text-[10px] text-slate-500 font-semibold tracking-wider uppercase mt-1 hidden sm:inline">Specification & Management Console</span>
      </div>
    </div>
    
    <!-- Right side tags & badges -->
    <div class="flex items-center gap-3 text-xs font-semibold">
      <div class="bg-blue-50 border border-blue-200 px-3 py-1.5 rounded-full text-blue-700 font-semibold text-[10px]">
        ${activePage.name}
      </div>
      <div class="hidden md:flex items-center gap-1.5 bg-slate-100 border border-slate-200 px-3 py-1.5 rounded-full text-slate-700">
        <span class="w-1.5 h-1.5 rounded-full bg-emerald-500 animate-pulse"></span>
        <span class="font-mono text-[10px]">VCS: Active</span>
      </div>
      <div class="hidden md:flex items-center gap-1.5 bg-slate-100 border border-slate-200 px-3 py-1.5 rounded-full text-slate-700">
        <span class="font-mono text-[10px] text-slate-400">Branch:</span>
        <span class="font-mono text-[10px] text-blue-600 font-bold">main</span>
      </div>
    </div>
  `;
  document.body.prepend(navbar);
  document.body.classList.add('gks-layout-active');
  
  // Apply dark theme class if on the dashboard page
  const isDashboard = activePage.filename === "index.html" || currentFilename === "" || currentFilename === "index.html";
  if (isDashboard) {
    document.body.classList.add('gks-dashboard-active');
  }

  // 5. Inject Mobile Backdrop
  const backdrop = document.createElement('div');
  backdrop.className = 'gks-backdrop';
  backdrop.id = 'gks-backdrop';
  document.body.appendChild(backdrop);

  // 6. Inject Unified Sidebar
  const sidebar = document.createElement('aside');
  sidebar.className = 'gks-sidebar';
  sidebar.id = 'gks-sidebar';
  sidebar.innerHTML = `
    <!-- Pages Navigation List -->
    <div class="gks-sidebar-section">
      <div class="gks-section-title">เมนูนำทางหลัก</div>
      <nav class="gks-nav-list" id="gks-page-list"></nav>
    </div>
    
    <!-- Table of Contents (TOC) Container -->
    <div class="gks-toc-section" id="gks-toc-container">
      <div class="gks-section-title" style="padding-left: 0.5rem;">สารบัญของหน้านี้</div>
      <div class="gks-toc-list" id="gks-toc-list"></div>
    </div>
    
    <!-- Sidebar Footer with Collapse Toggle -->
    <div class="gks-sidebar-footer">
      <button class="gks-toggle-btn" id="gks-collapse-toggle">
        <svg id="gks-collapse-icon" class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 19l-7-7 7-7m8 14l-7-7 7-7"></path></svg>
        <span class="gks-label" id="gks-collapse-label">ย่อแถบเมนู</span>
      </button>
      <div class="gks-sidebar-info">GKS/MSP Specification v2.3</div>
    </div>
  `;
  document.body.appendChild(sidebar);

  // 7. Populating Page Navigation Links
  const pageListContainer = document.getElementById('gks-page-list');
  PAGES.forEach(page => {
    const link = document.createElement('a');
    link.href = page.filename;
    // Check if current file name matches page filename
    const isCurrent = currentFilename.includes(page.filename) || ((currentFilename === "" || currentFilename === "index.html") && page.filename === "index.html");
    link.className = `gks-nav-link ${isCurrent ? 'active' : ''}`;
    link.innerHTML = `
      ${page.icon}
      <span class="gks-label truncate">${page.label || page.name}</span>
    `;
    pageListContainer.appendChild(link);
  });

  // 8. Find Content Container and adjust spacing dynamically
  const mainContainer = document.querySelector('main') || document.querySelector('section.flex-1');
  if (mainContainer) {
    mainContainer.classList.remove('lg:pl-[19.5rem]');
    mainContainer.classList.add('gks-main-container');
  }

  // 9. Build Sidebar Content based on Page Type
  const tocList = document.getElementById('gks-toc-list');
  
  if (isBacklogPage) {
    // BACKLOG PAGE SPECIAL INTEGRATION:
    // Move the old filter card and tab buttons inside the new sidebar!
    const oldAside = document.querySelector('aside');
    if (oldAside) {
      // Find the components we need to migrate
      const navCard = oldAside.querySelector('nav');
      const actionCard = oldAside.querySelector('.shadow-sm.space-y-2') || oldAside.querySelector('div:has(button[onclick*="openAddTaskModal"])');
      const filtersCard = oldAside.querySelector('#sidebar-filters-container');
      
      const tocContainer = document.getElementById('gks-toc-container');
      if (tocContainer) {
        tocContainer.innerHTML = ''; // Clear TOC placeholder contents
        tocContainer.style.padding = '1rem 0.5rem';
        
        // Helper function to dynamically theme elements for dark sidebar integration
        const themeCard = (element) => {
          if (!element) return;
          element.classList.remove('bg-white', 'border-slate-200', 'shadow-sm', 'p-4', 'p-5');
          element.classList.add('bg-slate-900/60', 'border', 'border-slate-800/80', 'rounded-xl', 'p-4.5', 'mb-4', 'text-slate-300');
          
          // Fix label texts
          element.querySelectorAll('label, h5').forEach(label => {
            label.classList.remove('text-slate-400', 'text-slate-500');
            label.classList.add('text-slate-400', 'font-bold');
            label.style.color = '#94a3b8';
          });
          
          // Theme standard buttons
          element.querySelectorAll('button').forEach(btn => {
            if (btn.classList.contains('border-slate-200')) {
              btn.classList.remove('border-slate-200', 'text-slate-700', 'bg-white');
              btn.classList.add('border-slate-800', 'text-slate-300', 'bg-slate-950/40', 'hover:bg-slate-950/80');
            }
          });

          // Theme select selectors and search fields
          element.querySelectorAll('select, input').forEach(field => {
            field.classList.remove('bg-slate-50', 'border-slate-200', 'text-slate-800');
            field.classList.add('bg-slate-950/60', 'border-slate-800', 'text-slate-200', 'w-full');
            field.style.color = '#cbd5e1';
            field.style.borderColor = '#1e293b';
          });
        };

        // Theme and append elements to sidebar
        if (navCard) {
          const wrapper = document.createElement('div');
          wrapper.className = 'mb-4';
          const title = document.createElement('div');
          title.className = 'gks-section-title px-2';
          title.textContent = 'แผงควบคุมบอร์ด';
          wrapper.appendChild(title);
          
          // Modify active/inactive buttons to fit dark style
          navCard.querySelectorAll('button').forEach(btn => {
            btn.classList.remove('pl-3', 'py-2.5', 'hover:bg-slate-100', 'text-slate-600', 'hover:text-slate-950');
            btn.classList.add('px-3', 'py-2', 'text-xs', 'rounded-lg', 'text-slate-400', 'hover:text-white', 'hover:bg-slate-900/60', 'w-full', 'text-left');
          });
          
          wrapper.appendChild(navCard);
          tocContainer.appendChild(wrapper);
        }

        if (actionCard) {
          const wrapper = document.createElement('div');
          themeCard(actionCard);
          wrapper.appendChild(actionCard);
          tocContainer.appendChild(wrapper);
        }

        if (filtersCard) {
          // Filters card needs to be appended and styled
          filtersCard.classList.remove('hidden'); // Keep active since we manage it in sidebar
          themeCard(filtersCard);
          tocContainer.appendChild(filtersCard);
        }
      }
    }
  } else {
    // STANDARD SPEC PAGES: Parse and generate TOC dynamically
    const sections = document.querySelectorAll('section');
    if (sections.length > 0) {
      sections.forEach(section => {
        if (!section.id) return;
        const heading = section.querySelector('h2') || section.querySelector('h3');
        if (!heading) return;

        const headingText = heading.textContent.trim();
        const link = document.createElement('a');
        link.href = `#${section.id}`;
        link.className = `gks-toc-link ${heading.tagName === 'H3' ? 'gks-toc-indent-3' : ''}`;
        link.textContent = headingText;
        tocList.appendChild(link);
      });

      // Implement dynamic Scroll Highlight Sync via IntersectionObserver
      const observerOptions = {
        root: null,
        rootMargin: '-10% 0px -80% 0px',
        threshold: 0
      };

      const observer = new IntersectionObserver(entries => {
        entries.forEach(entry => {
          if (entry.isIntersecting) {
            const activeId = entry.target.getAttribute('id');
            document.querySelectorAll('.gks-toc-link').forEach(link => {
              if (link.getAttribute('href') === `#${activeId}`) {
                link.classList.add('active');
                // Scroll TOC menu so active item is visible
                link.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
              } else {
                link.classList.remove('active');
              }
            });
          }
        });
      }, observerOptions);

      sections.forEach(section => {
        if (section.id) observer.observe(section);
      });
    } else {
      // If no sections exist on this page, hide TOC box
      document.getElementById('gks-toc-container').style.display = 'none';
    }
  }

  // 10. Sidebar Expand / Collapse Toggle Controller (Desktop)
  const collapseToggle = document.getElementById('gks-collapse-toggle');
  const collapseIcon = document.getElementById('gks-collapse-icon');
  const collapseLabel = document.getElementById('gks-collapse-label');
  
  // Read state from localStorage to maintain user preference across pages
  const isCollapsed = localStorage.getItem('gks-sidebar-collapsed') === 'true';
  if (isCollapsed) {
    sidebar.classList.add('collapsed');
    if (mainContainer) mainContainer.classList.add('collapsed');
    collapseIcon.innerHTML = `<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 5l7 7-7 7M5 5l7 7-7 7"></path>`; // Chevron pointing right
    collapseLabel.textContent = '';
  }

  collapseToggle.addEventListener('click', () => {
    const collapsing = sidebar.classList.toggle('collapsed');
    if (mainContainer) mainContainer.classList.toggle('collapsed');
    
    // Save preference
    localStorage.setItem('gks-sidebar-collapsed', collapsing);
    
    if (collapsing) {
      collapseIcon.innerHTML = `<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 5l7 7-7 7M5 5l7 7-7 7"></path>`;
      collapseLabel.textContent = '';
    } else {
      collapseIcon.innerHTML = `<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 19l-7-7 7-7m8 14l-7-7 7-7"></path>`;
      collapseLabel.textContent = 'ย่อแถบเมนู';
    }
  });

  // 11. Mobile Sidebar Controls
  const mobileToggle = document.getElementById('gks-mobile-toggle');
  
  const toggleMobileSidebar = () => {
    sidebar.classList.toggle('mobile-open');
    backdrop.classList.toggle('active');
  };

  mobileToggle.addEventListener('click', toggleMobileSidebar);
  backdrop.addEventListener('click', toggleMobileSidebar);

  // Close mobile sidebar on link clicks
  sidebar.querySelectorAll('.gks-nav-link').forEach(link => {
    link.addEventListener('click', () => {
      if (sidebar.classList.contains('mobile-open')) {
        toggleMobileSidebar();
      }
    });
  });
});
