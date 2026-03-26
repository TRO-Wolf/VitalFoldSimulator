# Project Origins

> Consolidated from: project.md, synthetic_data.md

---

## Original Project Prompt

Build a simulator project in Rust using the Actix framework, deployed to Render.com.
The simulator populates data based on a provided SQL schema.

**Long-term goal:** A data pipeline using AWS. This simulator generates realistic
healthcare data to power a portfolio project where data flows from micro-regions
across the country to a Florida-based healthcare company.

**Tech stack (human-specified):**
- Aurora DSQL for cost optimization and serverless
- Rust + Actix for performance
- Render.com for hosting

**General requirements:**
- Ability to turn the engine on/off through an API
- Basic API token authentication
- Fake name generation for patients
- DynamoDB integration (1 table: patient_visit with embedded vitals)

---

## Synthetic Data Definitions

### Insurance Companies (7)
1. Orange Spear
2. Care Medical
3. Cade Medical
4. Multiplied Health
5. Octi Care
6. Tatnay
7. Caymana

### Diagnosis Codes (8, cardiac-focused)
1. Atrial Fibrillation (AFib)
2. Coronary Artery Disease (CAD)
3. Chest Pain
4. Hypertension
5. Hyperlipidemia
6. Shortness of Breath (SOB)
7. Tachycardia
8. Bradycardia

### Heart Clinic Locations (10)
| City | State | Count |
|------|-------|-------|
| Charlotte | NC | 1 |
| Asheville | NC | 1 |
| Atlanta | GA | 2 |
| Tallahassee | FL | 1 |
| Miami | FL | 2 |
| Orlando | FL | 1 |
| Jacksonville | FL | 2 |

Florida dominates (6 clinics) — company HQ is **Vital Fold Health LLC** based in FL.

### Provider & Patient Names
Generated randomly via the `fake` crate. Names are intentionally obviously random.
