-- Abstract symbolic action semantics. Numerical channel fitting is outside this proof.
inductive Action where
  | heat | press | cool
  deriving DecidableEq
abbrev Program := List Action

def run {S : Type} (step : S → Action → S) (state : S) : Program → S
  | [] => state
  | action :: rest => run step (step state action) rest

theorem composition {S : Type} (step : S → Action → S) (s : S)
    (p q : Program) : run step s (p ++ q) = run step (run step s p) q := by
  induction p generalizing s with
  | nil => rfl
  | cons a rest ih => exact ih (step s a)

def Valid (p : Program) : Prop := 0 < p.length ∧ p.length ≤ 3

theorem bounded_composition (p q : Program)
    (hp : 0 < p.length) (bound : p.length + q.length ≤ 3) : Valid (p ++ q) := by
  constructor
  · simp only [List.length_append]
    omega
  · simpa only [List.length_append] using bound

-- An observation and a prediction are different constructors; neither grants promotion.
inductive Evidence where
  | observed : Nat → Evidence
  | predicted : Nat → Nat → Evidence

theorem prediction_is_not_observation (revision outcome actual : Nat) :
    Evidence.predicted revision outcome ≠ Evidence.observed actual := by
  intro h
  cases h

-- Reference policy for working hypotheses, separate from numerical competence.
inductive WorkingStatus where
  | candidate | supported | contested
  deriving DecidableEq

def classify (support counterexamples : Nat) (crossCondition : Bool) : WorkingStatus :=
  if counterexamples > 0 then .contested
  else if support >= 3 ∧ crossCondition = true then .supported
  else .candidate

theorem counterexample_contests (s c : Nat) (cross : Bool) (h : c > 0) :
    classify s c cross = .contested := by
  simp [classify, h]

theorem working_support_has_no_counterexamples (s c : Nat) (cross : Bool)
    (h : classify s c cross = .supported) : c = 0 := by
  cases c with
  | zero => rfl
  | succ n => simp [classify] at h
