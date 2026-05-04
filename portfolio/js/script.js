'use strict';

// Nav scroll effect
const nav = document.getElementById('nav');
window.addEventListener('scroll', () => {
  nav.classList.toggle('scrolled', window.scrollY > 20);
}, { passive: true });

// Mobile nav toggle
const navToggle = document.getElementById('navToggle');
const navLinks  = document.getElementById('navLinks');
navToggle.addEventListener('click', () => {
  navLinks.classList.toggle('open');
  const open = navLinks.classList.contains('open');
  navToggle.setAttribute('aria-expanded', open);
});
navLinks.querySelectorAll('a').forEach(link => {
  link.addEventListener('click', () => navLinks.classList.remove('open'));
});

// Typing animation for hero title
const titles = [
  'Senior Backend Developer',
  'Java / Spring Boot Specialist',
  'Cloud & Kubernetes Engineer',
  'Microservices Architect',
  'AI Agent Builder',
];
let titleIdx = 0, charIdx = 0, deleting = false;
const titleEl = document.getElementById('heroTitle');

function typeTitle() {
  if (!titleEl) return;
  const current = titles[titleIdx];

  if (deleting) {
    charIdx--;
    titleEl.textContent = current.slice(0, charIdx);
    if (charIdx === 0) {
      deleting = false;
      titleIdx = (titleIdx + 1) % titles.length;
      setTimeout(typeTitle, 400);
      return;
    }
    setTimeout(typeTitle, 40);
  } else {
    charIdx++;
    titleEl.textContent = current.slice(0, charIdx);
    if (charIdx === current.length) {
      deleting = true;
      setTimeout(typeTitle, 2400);
      return;
    }
    setTimeout(typeTitle, 65);
  }
}
setTimeout(typeTitle, 800);

// Scroll reveal
const observer = new IntersectionObserver((entries) => {
  entries.forEach((entry, i) => {
    if (entry.isIntersecting) {
      setTimeout(() => entry.target.classList.add('visible'), i * 80);
      observer.unobserve(entry.target);
    }
  });
}, { threshold: 0.12 });

document.querySelectorAll(
  '.stat-card, .skill-group, .timeline__card, .project-card, .edu-card, .cert-card, .contact__item'
).forEach(el => {
  el.classList.add('reveal');
  observer.observe(el);
});

// Contact form — mailto fallback (no backend needed)
const form = document.getElementById('contactForm');
if (form) {
  form.addEventListener('submit', e => {
    e.preventDefault();
    const name    = form.name.value.trim();
    const email   = form.email.value.trim();
    const subject = form.subject.value.trim() || 'Contact from Portfolio';
    const message = form.message.value.trim();

    if (!name || !email || !message) {
      alert('Please fill in your name, email, and message.');
      return;
    }

    const body = `Hi William,\n\nMy name is ${name} (${email}).\n\n${message}`;
    const mailto = `mailto:wlopezob@gmail.com?subject=${encodeURIComponent(subject)}&body=${encodeURIComponent(body)}`;
    window.location.href = mailto;
  });
}

// Smooth active-nav highlighting
const sections = document.querySelectorAll('section[id]');
const navAnchors = document.querySelectorAll('.nav__links a[href^="#"]');

const sectionObserver = new IntersectionObserver((entries) => {
  entries.forEach(entry => {
    if (entry.isIntersecting) {
      navAnchors.forEach(a => a.classList.remove('active'));
      const active = document.querySelector(`.nav__links a[href="#${entry.target.id}"]`);
      if (active) active.classList.add('active');
    }
  });
}, { rootMargin: '-40% 0px -55% 0px' });

sections.forEach(s => sectionObserver.observe(s));
