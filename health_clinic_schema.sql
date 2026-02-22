CREATE SCHEMA IF NOT EXISTS vital_fold;


--=============================================================================
--=============================================================================
-- TABLE CREATION
--=============================================================================
--=============================================================================
CREATE TABLE IF NOT EXISTS vital_fold.insurance_company (
    company_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    company_name VARCHAR(255) NOT NULL,
    email VARCHAR(255) NOT NULL,
    phone_number VARCHAR(20) NOT NULL,
    tax_id_number INT NOT NULL
);

--=============================================================================
-- INDEX DEFINITIONS
--=============================================================================
CREATE INDEX ASYNC IF NOT EXISTS idx_vt_insurance_company_name
    ON vital_fold.insurance_company (
        company_name
    );

CREATE INDEX ASYNC IF NOT EXISTS idx_vt_insurance_company_email
    ON vital_fold.insurance_company (
        email
    );

CREATE INDEX ASYNC IF NOT EXISTS idx_vt_insurance_company_tax_id_number
    ON vital_fold.insurance_company (
        tax_id_number
    );


--=============================================================================
--=============================================================================
-- TABLE CREATION
--=============================================================================
--=============================================================================
CREATE TABLE IF NOT EXISTS vital_fold.insurance_plan (
    insurance_plan_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    plan_name VARCHAR(255) NOT NULL,
    company_id UUID NOT NULL,
    deductible_amount DECIMAL(10, 2) NOT NULL,
    copay_amount DECIMAL(10, 2) NOT NULL,
    prior_auth_required BOOLEAN NOT NULL,
    active_plan BOOLEAN NOT NULL,
    active_start_date DATE NOT NULL,
    FOREIGN KEY (company_id) REFERENCES vital_fold.insurance_company(company_id)
);

--=============================================================================
-- INDEX DEFINITIONS
--=============================================================================
CREATE INDEX ASYNC IF NOT EXISTS idx_vt_insurance_plan_company_id
    ON vital_fold.insurance_plan (
        company_id
    );

CREATE INDEX ASYNC IF NOT EXISTS idx_vt_insurance_plan_name
    ON vital_fold.insurance_plan (
        plan_name
    );

CREATE INDEX ASYNC IF NOT EXISTS idx_vt_insurance_plan_active
    ON vital_fold.insurance_plan (
        active_plan
    );

CREATE INDEX ASYNC IF NOT EXISTS idx_vt_insurance_plan_start_date
    ON vital_fold.insurance_plan (
        active_start_date
    );

CREATE INDEX ASYNC IF NOT EXISTS idx_vt_insurance_plan_company_active
    ON vital_fold.insurance_plan (
        company_id,
        active_plan
    );


--=============================================================================
--=============================================================================
-- TABLE CREATION
--=============================================================================
--=============================================================================
CREATE TABLE IF NOT EXISTS vital_fold.provider (
    provider_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    first_name VARCHAR(255) NOT NULL,
    last_name VARCHAR(255) NOT NULL,
    specialty VARCHAR(255) NOT NULL,
    license_type VARCHAR(255) NOT NULL,
    phone_number VARCHAR(20) NOT NULL,
    email VARCHAR(255) NOT NULL
);

--=============================================================================
-- INDEX DEFINITIONS
--=============================================================================
CREATE INDEX ASYNC IF NOT EXISTS idx_vt_provider_last_name
    ON vital_fold.provider (
        last_name
    );

CREATE INDEX ASYNC IF NOT EXISTS idx_vt_provider_specialty
    ON vital_fold.provider (
        specialty
    );

CREATE INDEX ASYNC IF NOT EXISTS idx_vt_provider_email
    ON vital_fold.provider (
        email
    );

CREATE INDEX ASYNC IF NOT EXISTS idx_vt_provider_name
    ON vital_fold.provider (
        last_name,
        first_name
    );


--=============================================================================
--=============================================================================
-- TABLE CREATION
--=============================================================================
--=============================================================================
CREATE TABLE IF NOT EXISTS vital_fold.clinic (
    clinic_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    clinic_name VARCHAR(255) NOT NULL,
    region VARCHAR(255) NOT NULL,
    street_address VARCHAR(255) NOT NULL,
    city VARCHAR(255) NOT NULL,
    state VARCHAR(255) NOT NULL,
    zip_code VARCHAR(10) NOT NULL,
    phone_number VARCHAR(20) NOT NULL,
    email VARCHAR(255) NOT NULL
);

--=============================================================================
-- INDEX DEFINITIONS
--=============================================================================
CREATE INDEX ASYNC IF NOT EXISTS idx_vt_clinic_name
    ON vital_fold.clinic (
        clinic_name
    );

CREATE INDEX ASYNC IF NOT EXISTS idx_vt_clinic_region
    ON vital_fold.clinic (
        region
    );

CREATE INDEX ASYNC IF NOT EXISTS idx_vt_clinic_city
    ON vital_fold.clinic (
        city
    );

CREATE INDEX ASYNC IF NOT EXISTS idx_vt_clinic_state
    ON vital_fold.clinic (
        state
    );

CREATE INDEX ASYNC IF NOT EXISTS idx_vt_clinic_email
    ON vital_fold.clinic (
        email
    );


--=============================================================================
--=============================================================================
-- TABLE CREATION (patient must come before tables that reference it)
--=============================================================================
--=============================================================================
CREATE TABLE IF NOT EXISTS vital_fold.patient (
    patient_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    first_name VARCHAR(255) NOT NULL,
    last_name VARCHAR(255) NOT NULL,
    middle_name VARCHAR(255),
    date_of_birth DATE NOT NULL,
    street_address VARCHAR(255) NOT NULL,
    city VARCHAR(255) NOT NULL,
    state VARCHAR(255) NOT NULL,
    zip_code VARCHAR(10) NOT NULL,
    phone_number VARCHAR(20) NOT NULL,
    email VARCHAR(255) NOT NULL,
    registration_date DATE NOT NULL,
    emergency_contact_id VARCHAR(255) NOT NULL
);

--=============================================================================
-- INDEX DEFINITIONS
--=============================================================================
CREATE INDEX ASYNC IF NOT EXISTS idx_vt_patient_last_name
    ON vital_fold.patient (
        last_name
    );

CREATE INDEX ASYNC IF NOT EXISTS idx_vt_patient_email
    ON vital_fold.patient (
        email
    );

CREATE INDEX ASYNC IF NOT EXISTS idx_vt_patient_phone_number
    ON vital_fold.patient (
        phone_number
    );

CREATE INDEX ASYNC IF NOT EXISTS idx_vt_patient_date_of_birth
    ON vital_fold.patient (
        date_of_birth
    );

CREATE INDEX ASYNC IF NOT EXISTS idx_vt_patient_registration_date
    ON vital_fold.patient (
        registration_date
    );

CREATE INDEX ASYNC IF NOT EXISTS idx_vt_patient_name
    ON vital_fold.patient (
        last_name,
        first_name
    );


--=============================================================================
--=============================================================================
-- TABLE CREATION
--=============================================================================
--=============================================================================
CREATE TABLE IF NOT EXISTS vital_fold.emergency_contact (
    emergency_contact_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    patient_id UUID NOT NULL,
    first_name VARCHAR(255) NOT NULL,
    last_name VARCHAR(255) NOT NULL,
    relationship VARCHAR(255) NOT NULL,
    phone_number VARCHAR(20) NOT NULL,
    email VARCHAR(255) NOT NULL,
    FOREIGN KEY (patient_id) REFERENCES vital_fold.patient(patient_id)
);

--=============================================================================
-- INDEX DEFINITIONS
--=============================================================================
CREATE INDEX ASYNC IF NOT EXISTS idx_vt_emergency_contact_patient_id
    ON vital_fold.emergency_contact (
        patient_id
    );

CREATE INDEX ASYNC IF NOT EXISTS idx_vt_emergency_contact_phone_number
    ON vital_fold.emergency_contact (
        phone_number
    );

CREATE INDEX ASYNC IF NOT EXISTS idx_vt_emergency_contact_email
    ON vital_fold.emergency_contact (
        email
    );


--=============================================================================
--=============================================================================
-- TABLE CREATION
--=============================================================================
--=============================================================================
CREATE TABLE IF NOT EXISTS vital_fold.patient_demographics (
    demographics_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    patient_id UUID NOT NULL,
    first_name VARCHAR(255) NOT NULL,
    last_name VARCHAR(255) NOT NULL,
    date_of_birth DATE NOT NULL,
    age INT NOT NULL,
    ssn VARCHAR(11) NOT NULL,
    ethnicity VARCHAR(255) NOT NULL,
    birth_gender VARCHAR(50) NOT NULL,
    FOREIGN KEY (patient_id) REFERENCES vital_fold.patient(patient_id)
);

--=============================================================================
-- INDEX DEFINITIONS
--=============================================================================
CREATE INDEX ASYNC IF NOT EXISTS idx_vt_patient_demographics_patient_id
    ON vital_fold.patient_demographics (
        patient_id
    );

CREATE INDEX ASYNC IF NOT EXISTS idx_vt_patient_demographics_ssn
    ON vital_fold.patient_demographics (
        ssn
    );

CREATE INDEX ASYNC IF NOT EXISTS idx_vt_patient_demographics_date_of_birth
    ON vital_fold.patient_demographics (
        date_of_birth
    );


--=============================================================================
--=============================================================================
-- TABLE CREATION
--=============================================================================
--=============================================================================
CREATE TABLE IF NOT EXISTS vital_fold.patient_insurance (
    patient_insurance_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    patient_id UUID NOT NULL,
    insurance_plan_id UUID NOT NULL,
    policy_number VARCHAR(255) NOT NULL,
    coverage_start_date DATE NOT NULL,
    coverage_end_date DATE,
    FOREIGN KEY (patient_id) REFERENCES vital_fold.patient(patient_id),
    FOREIGN KEY (insurance_plan_id) REFERENCES vital_fold.insurance_plan(insurance_plan_id)
);

--=============================================================================
-- INDEX DEFINITIONS
--=============================================================================
CREATE INDEX ASYNC IF NOT EXISTS idx_vt_patient_insurance_patient_id
    ON vital_fold.patient_insurance (
        patient_id
    );

CREATE INDEX ASYNC IF NOT EXISTS idx_vt_patient_insurance_insurance_plan_id
    ON vital_fold.patient_insurance (
        insurance_plan_id
    );

CREATE INDEX ASYNC IF NOT EXISTS idx_vt_patient_insurance_policy_number
    ON vital_fold.patient_insurance (
        policy_number
    );

CREATE INDEX ASYNC IF NOT EXISTS idx_vt_patient_insurance_coverage_start_date
    ON vital_fold.patient_insurance (
        coverage_start_date
    );

CREATE INDEX ASYNC IF NOT EXISTS idx_vt_patient_insurance_coverage_end_date
    ON vital_fold.patient_insurance (
        coverage_end_date
    );


--=============================================================================
--=============================================================================
-- TABLE CREATION
--=============================================================================
--=============================================================================
CREATE TABLE IF NOT EXISTS vital_fold.clinic_schedule (
    schedule_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    clinic_id UUID NOT NULL,
    provider_id UUID NOT NULL,
    day_of_week VARCHAR(20) NOT NULL,
    start_time TIME NOT NULL,
    end_time TIME NOT NULL,
    FOREIGN KEY (clinic_id) REFERENCES vital_fold.clinic(clinic_id),
    FOREIGN KEY (provider_id) REFERENCES vital_fold.provider(provider_id)
);

--=============================================================================
-- INDEX DEFINITIONS
--=============================================================================
CREATE INDEX ASYNC IF NOT EXISTS idx_vt_clinic_schedule_clinic_id
    ON vital_fold.clinic_schedule (
        clinic_id
    );

CREATE INDEX ASYNC IF NOT EXISTS idx_vt_clinic_schedule_provider_id
    ON vital_fold.clinic_schedule (
        provider_id
    );

CREATE INDEX ASYNC IF NOT EXISTS idx_vt_clinic_schedule_day_of_week
    ON vital_fold.clinic_schedule (
        day_of_week
    );

CREATE INDEX ASYNC IF NOT EXISTS idx_vt_clinic_schedule_clinic_day
    ON vital_fold.clinic_schedule (
        clinic_id,
        day_of_week
    );

CREATE INDEX ASYNC IF NOT EXISTS idx_vt_clinic_schedule_provider_day
    ON vital_fold.clinic_schedule (
        provider_id,
        day_of_week
    );


--=============================================================================
--=============================================================================
-- TABLE CREATION
--=============================================================================
--=============================================================================
CREATE TABLE IF NOT EXISTS vital_fold.appointment (
    appointment_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    patient_id UUID NOT NULL,
    provider_id UUID NOT NULL,
    clinic_id UUID NOT NULL,
    appointment_date TIMESTAMP NOT NULL,
    reason_for_visit VARCHAR(255) NOT NULL,
    FOREIGN KEY (patient_id) REFERENCES vital_fold.patient(patient_id),
    FOREIGN KEY (provider_id) REFERENCES vital_fold.provider(provider_id),
    FOREIGN KEY (clinic_id) REFERENCES vital_fold.clinic(clinic_id)
);

--=============================================================================
-- INDEX DEFINITIONS
--=============================================================================
CREATE INDEX ASYNC IF NOT EXISTS idx_vt_appointment_patient_id
    ON vital_fold.appointment (
        patient_id
    );

CREATE INDEX ASYNC IF NOT EXISTS idx_vt_appointment_provider_id
    ON vital_fold.appointment (
        provider_id
    );

CREATE INDEX ASYNC IF NOT EXISTS idx_vt_appointment_clinic_id
    ON vital_fold.appointment (
        clinic_id
    );

CREATE INDEX ASYNC IF NOT EXISTS idx_vt_appointment_date
    ON vital_fold.appointment (
        appointment_date
    );

CREATE INDEX ASYNC IF NOT EXISTS idx_vt_appointment_provider_date
    ON vital_fold.appointment (
        provider_id,
        appointment_date
    );

CREATE INDEX ASYNC IF NOT EXISTS idx_vt_appointment_clinic_date
    ON vital_fold.appointment (
        clinic_id,
        appointment_date
    );


--=============================================================================
--=============================================================================
-- TABLE CREATION
--=============================================================================
--=============================================================================
CREATE TABLE IF NOT EXISTS vital_fold.medical_record (
    medical_record_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    patient_id UUID NOT NULL,
    provider_id UUID NOT NULL,
    clinic_id UUID NOT NULL,
    record_date TIMESTAMP NOT NULL,
    diagnosis VARCHAR(255) NOT NULL,
    treatment VARCHAR(255) NOT NULL,
    FOREIGN KEY (patient_id) REFERENCES vital_fold.patient(patient_id),
    FOREIGN KEY (provider_id) REFERENCES vital_fold.provider(provider_id),
    FOREIGN KEY (clinic_id) REFERENCES vital_fold.clinic(clinic_id)
);

--=============================================================================
-- INDEX DEFINITIONS
--=============================================================================
CREATE INDEX ASYNC IF NOT EXISTS idx_vt_medical_record_patient_id
    ON vital_fold.medical_record (
        patient_id
    );

CREATE INDEX ASYNC IF NOT EXISTS idx_vt_medical_record_provider_id
    ON vital_fold.medical_record (
        provider_id
    );

CREATE INDEX ASYNC IF NOT EXISTS idx_vt_medical_record_clinic_id
    ON vital_fold.medical_record (
        clinic_id
    );

CREATE INDEX ASYNC IF NOT EXISTS idx_vt_medical_record_date
    ON vital_fold.medical_record (
        record_date
    );

CREATE INDEX ASYNC IF NOT EXISTS idx_vt_medical_record_patient_date
    ON vital_fold.medical_record (
        patient_id,
        record_date
    );
