---
title: "Agent & Skill Index"
type: INDEX
id: "index-agents"
created: 2025-12-01
updated: 2025-12-01
agent: copilot
model: gemini-3-pro
requested_by: system
summary: |
  Index of available agent roles and skills for dynamic equipping.
keywords: [agents, index, skills, roles]
tags: ["#index", "#agents", "#skills"]
project: Git-Core-Protocol
---

# 🧠 Agent & Skill Index

## 🚦 Routing Logic

When the user prompts for a task, identify the **Domain** and **Role**.
Then, run the `equip-agent` script to load that persona.

---

## 📂 Domain: Engineering

| Role | Description | Source Recipe | Recommended Skills |
|------|-------------|---------------|-------------------|
| **AI Engineer** | LLM integration, prompt engineering | `engineering/ai-engineer.md` | `prompt-tuning`, `llm-eval` |
| **Backend Architect** | System design, DB schema, API structure | `engineering/backend-architect.md` | `system-design`, `db-schema` |
| **DevOps Automator** | CI/CD, Docker, Infrastructure | `engineering/devops-automator.md` | `docker`, `github-actions` |
| **Frontend Dev** | UI implementation, React/Vue, CSS | `engineering/frontend-developer.md` | `react-best-practices`, `css-modules` |
| **Mobile App Builder** | iOS/Android development | `engineering/mobile-app-builder.md` | `react-native`, `flutter` |
| **Rapid Prototyper** | MVP creation, speed coding | `engineering/rapid-prototyper.md` | `mvp-hacks`, `speed-coding` |
| **Test Fixer** | Writing tests, debugging failures | `engineering/test-writer-fixer.md` | `jest`, `pytest` |

---

## 📂 Domain: Product & Design

| Role | Description | Source Recipe | Recommended Skills |
|------|-------------|---------------|-------------------|
| **Brand Guardian** | Brand consistency, tone of voice | `design/brand-guardian.md` | `brand-guidelines` |
| **Feedback Synthesizer** | User feedback analysis | `product/feedback-synthesizer.md` | `sentiment-analysis` |
| **Sprint Prioritizer** | Backlog management, sprint planning | `product/sprint-prioritizer.md` | `agile-scrum` |
| **Trend Researcher** | Market analysis, competitor research | `product/trend-researcher.md` | `market-research` |
| **UI Designer** | Visual interface design | `design/ui-designer.md` | `figma`, `tailwind` |
| **UX Researcher** | User flows, empathy mapping | `design/ux-researcher.md` | `user-interviews` |
| **Visual Storyteller** | Narrative design, visual communication | `design/visual-storyteller.md` | `storytelling` |
| **Whimsy Injector** | Adding delight and fun details | `design/whimsy-injector.md` | `micro-interactions` |

---

## 📂 Domain: Marketing

| Role | Description | Source Recipe | Recommended Skills |
|------|-------------|---------------|-------------------|
| **App Store Optimizer** | ASO, keyword optimization | `marketing/app-store-optimizer.md` | `aso-tools` |
| **Content Creator** | Documentation, tutorials, blogs | `marketing/content-creator.md` | `technical-writing`, `seo` |
| **Growth Hacker** | User acquisition, viral loops | `marketing/growth-hacker.md` | `analytics`, `ab-testing` |
| **Instagram Curator** | Social media visual strategy | `marketing/instagram-curator.md` | `social-media` |
| **Reddit Builder** | Community engagement | `marketing/reddit-community-builder.md` | `community-management` |
| **TikTok Strategist** | Short-form video strategy | `marketing/tiktok-strategist.md` | `video-trends` |
| **Twitter Engager** | Social listening and response | `marketing/twitter-engager.md` | `social-listening` |

---

## 📂 Domain: Project Management

| Role | Description | Source Recipe | Recommended Skills |
|------|-------------|---------------|-------------------|
| **Experiment Tracker** | Hypothesis testing, metrics | `project-management/experiment-tracker.md` | `data-analysis` |
| **Project Shipper** | Release management, deployment | `project-management/project-shipper.md` | `release-management` |
| **Studio Producer** | Team coordination, unblocking | `project-management/studio-producer.md` | `coordination` |

---

## 📂 Domain: Studio Operations

| Role | Description | Source Recipe | Recommended Skills |
|------|-------------|---------------|-------------------|
| **Analytics Reporter** | KPI tracking, reporting | `studio-operations/analytics-reporter.md` | `data-viz` |
| **Finance Tracker** | Budgeting, cost analysis | `studio-operations/finance-tracker.md` | `excel`, `finance` |
| **Infra Maintainer** | Tooling, environment health | `studio-operations/infrastructure-maintainer.md` | `sysadmin` |
| **Legal Checker** | Compliance, license verification | `studio-operations/legal-compliance-checker.md` | `compliance` |
| **Support Responder** | Customer support, ticket handling | `studio-operations/support-responder.md` | `zendesk` |

---

## 📂 Domain: Testing & QA

| Role | Description | Source Recipe | Recommended Skills |
|------|-------------|---------------|-------------------|
| **API Tester** | Endpoint verification, load testing | `testing/api-tester.md` | `postman`, `k6` |
| **Perf Benchmarker** | Speed and resource analysis | `testing/performance-benchmarker.md` | `profiling` |
| **Results Analyzer** | Test report interpretation | `testing/test-results-analyzer.md` | `log-analysis` |
| **Tool Evaluator** | Library/Tool selection | `testing/tool-evaluator.md` | `tech-radar` |
| **Workflow Optimizer** | Process improvement | `testing/workflow-optimizer.md` | `ci-cd-optimization` |

---

## 📂 Domain: Bonus

| Role | Description | Source Recipe | Recommended Skills |
|------|-------------|---------------|-------------------|
<!-- | **Joker** | Comic relief, easter eggs | `bonus/joker.md` | `humor` | -->
| **Studio Coach** | Team morale, conflict resolution | `bonus/studio-coach.md` | `coaching` |

---

## 🛠️ Skill Definitions (Capabilities)

*Skills are appended to the agent's context when loaded.*

- **Atomic Commits:** Enforces the Git-Core Protocol commit style.
- **Architecture Check:** Validates against `.gitcore/ARCHITECTURE.md`.
- **POML Parser:** Ability to understand Prompt Object Markup Language.

---

## 🚀 How to Activate

Run in terminal:

```powershell
./scripts/equip-agent.ps1 -Role "Backend Architect"
```

