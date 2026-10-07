<script lang="ts">
  import { goto } from '$app/navigation';
</script>

<div class="screen">
  <button class="back" on:click={() => history.back()}>‹ Retour</button>
  <h2 style="font-size:20px">Politique de confidentialité</h2>
  <div class="summary" style="margin-top:12px;line-height:1.6">
    <p>Conformément à la <strong>Loi n° 18-07</strong> du 10 juin 2018 relative à la protection
      des personnes physiques dans le traitement des données à caractère personnel, SOGISL
      traite vos données uniquement pour la vente de billets et le contrôle d'accès au
      Stade Ali Ammar.</p>
    <h3 style="font-size:15px;margin-top:10px">Données collectées</h3>
    <p class="mut">Identité (nom, e-mail), informations de compte, et le jeton de paiement
      fourni par la passerelle agréée. <strong>Aucune donnée de carte (PAN/CVV) n'est stockée.</strong></p>
    <h3 style="font-size:15px;margin-top:10px">Vos droits</h3>
    <p class="mut">Vous disposez d'un droit d'accès, de rectification et de suppression de vos
      données, exerçables depuis <a href="/me">votre espace</a> (modifier le profil, supprimer
      le compte).</p>
    <h3 style="font-size:15px;margin-top:10px">Suppression du compte et des données</h3>
    <p class="mut">Pour supprimer votre compte et les données associées (identité, compte,
      historique d'achats), suivez les étapes détaillées sur la page
      <a href="/supprimer-compte"><strong>Supprimer mon compte</strong></a>.</p>
    <h3 style="font-size:15px;margin-top:10px">Conservation</h3>
    <p class="mut">Les données sont conservées pour la durée nécessaire puis anonymisées ou
      supprimées. L'hébergement est réalisé sur le territoire national.</p>
    <h3 style="font-size:15px;margin-top:10px">Contact</h3>
    <p class="mut">Pour toute question relative à vos données : SOGISL, Stade Ali Ammar, Douira.
      Une déclaration est effectuée auprès de l'ANPDP.</p>
  </div>
</div>
