// Showcase filter, lightbox, copy buttons and scroll reveal. No dependencies.

const gallery = document.getElementById('gallery');
const filters = document.querySelectorAll('.filters button');

filters.forEach((btn) => {
  btn.addEventListener('click', () => {
    const f = btn.dataset.filter;
    filters.forEach((b) => b.setAttribute('aria-pressed', String(b === btn)));
    gallery.classList.toggle('filtered', f !== 'all');
    gallery.querySelectorAll('.shot').forEach((shot) => {
      shot.hidden = f !== 'all' && shot.dataset.cat !== f;
    });
  });
});

const box = document.getElementById('lightbox');
const boxImg = box.querySelector('img');
const boxCap = box.querySelector('p');

gallery.querySelectorAll('.shot button').forEach((btn) => {
  btn.addEventListener('click', () => {
    const img = btn.querySelector('img');
    const title = btn.parentElement.querySelector('strong').textContent;
    const meta = btn.parentElement.querySelector('figcaption span').textContent;
    boxImg.src = img.src;
    boxImg.alt = img.alt;
    boxCap.textContent = `${title} · ${meta}`;
    box.showModal();
  });
});
box.querySelector('.close').addEventListener('click', () => box.close());
box.addEventListener('click', (e) => { if (e.target === box) box.close(); });

document.querySelectorAll('.code').forEach((block) => {
  const btn = document.createElement('button');
  btn.type = 'button';
  btn.className = 'copy';
  btn.textContent = 'Copy';
  btn.addEventListener('click', async () => {
    try {
      await navigator.clipboard.writeText(block.querySelector('code').textContent);
      btn.textContent = 'Copied';
    } catch {
      btn.textContent = 'Select & copy';
    }
    setTimeout(() => { btn.textContent = 'Copy'; }, 1600);
  });
  block.appendChild(btn);
});

const reveals = document.querySelectorAll('.reveal');
if ('IntersectionObserver' in window) {
  const io = new IntersectionObserver((entries) => {
    entries.forEach((e) => {
      if (e.isIntersecting) { e.target.classList.add('in'); io.unobserve(e.target); }
    });
  }, { rootMargin: '0px 0px -10% 0px' });
  reveals.forEach((el) => io.observe(el));
} else {
  reveals.forEach((el) => el.classList.add('in'));
}
